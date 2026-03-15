//! Floem `Renderer` trait implementation backed by the D3D11 BatchRenderer.
//!
//! Text rasterization uses swash (consuming parley's pre-shaped glyph runs).
//! Shape rendering detects known primitives (rect, rounded rect, circle, line)
//! and routes them to the GPU SDF pipeline; arbitrary paths fall back to
//! tiny-skia CPU rasterization.

use std::collections::HashMap;

use floem_renderer::{Renderer, tiny_skia};
use floem_renderer::text::{Glyph, GlyphRunProps};

use floem_renderer::tiny_skia::{PathBuilder, Pixmap};
use floem_renderer::usvg::Transform;
use parley::swash;
use peniko::kurbo::{self, Affine, Point, Rect, RoundedRectRadii, Shape, Size};
use peniko::{BlendMode, BrushRef};

use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;

use win_renderer::{Error, HRESULT, Result, render};
use win_renderer::ID3D11Buffer;
use win_renderer::D3D11_VIEWPORT;
use win_renderer::glyph_atlas::{
    CachedGlyph, GlyphAtlas, SUBPX_BINS,
    create_system_correction_cbuffer,
};
use win_renderer::gpu::GpuDevice;
use win_renderer::renderer::BatchRenderer;
use win_renderer::texture::GpuTexture;
use win_renderer::util::{LayerCadence, LayerId};


// ── Glyph cache key ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SwashGlyphKey {
    font_blob_id: u64,
    font_index: u32,
    glyph_id: u16,
    font_size_bits: u32,
    x_bin: u8,
    y_bin: u8,
}

impl SwashGlyphKey {
    fn new(
        font_blob_id: u64,
        font_index: u32,
        glyph_id: u16,
        font_size: f32,
        x: f32,
        y: f32,
    ) -> (Self, f32, f32) {
        let x_floor = x.floor();
        let y_floor = y.floor();
        let x_fract = x - x_floor;
        let y_fract = y - y_floor;
        let x_bin = (x_fract * SUBPX_BINS as f32).min(SUBPX_BINS as f32 - 1.0) as u8;
        let y_bin = (y_fract * SUBPX_BINS as f32).min(SUBPX_BINS as f32 - 1.0) as u8;

        (
            Self {
                font_blob_id,
                font_index,
                glyph_id,
                font_size_bits: font_size.to_bits(),
                x_bin,
                y_bin,
            },
            x_floor + (x_bin as f32) / SUBPX_BINS as f32,
            y_floor + (y_bin as f32) / SUBPX_BINS as f32,
        )
    }
}


// ── SVG/Image caches ─────────────────────────────────────────────────────

struct CachedImage {
    texture_key: u32,
    #[allow(dead_code)]
    gpu_texture: GpuTexture,
}


// ── WinRenderer ────────────────────────────────────────────────────────

pub struct WinRenderer {
    gpu: GpuDevice,
    layers: CompositionLayerManager,
    layer_id: LayerId,
    viewport_size: (u32, u32),
    scale: f64,

    batch: BatchRenderer,
    atlas: GlyphAtlas,
    glyph_cache: HashMap<SwashGlyphKey, CachedGlyph>,
    scale_context: ScaleContext,
    correction_cbuffer: ID3D11Buffer,

    // Caches keyed by hash bytes
    svg_cache: HashMap<Vec<u8>, CachedImage>,
    img_cache: HashMap<Vec<u8>, CachedImage>,

    // Layer stack for push_layer/pop_layer (simplified: alpha only)
    alpha_stack: Vec<f32>,
    current_alpha: f32,

    capture: bool,
}

impl WinRenderer {
    /// Create a new WinRenderer from a raw Win32 HWND (as isize).
    pub fn new(hwnd_raw: isize, w: u32, h: u32, scale: f64) -> Result<Self> {
        let hwnd = windows::Win32::Foundation::HWND(hwnd_raw as *mut _);

        let gpu = GpuDevice::new()?;
        let mut layers = CompositionLayerManager::new(&gpu.dxgi_device, hwnd)?;
        let layer_id = layers.create_render_layer(
            &gpu,
            w.max(1),
            h.max(1),
            LayerCadence::OnDirty,
            true, // opaque
        )?;
        layers.commit()?;

        let batch = BatchRenderer::new(&gpu.device)?;
        let atlas = GlyphAtlas::new(&gpu.device)?;
        let correction_cbuffer = create_system_correction_cbuffer(&gpu.device)?;

        Ok(Self {
            gpu,
            layers,
            layer_id,
            viewport_size: (w.max(1), h.max(1)),
            scale,
            batch,
            atlas,
            glyph_cache: HashMap::new(),
            scale_context: ScaleContext::new(),
            correction_cbuffer,
            svg_cache: HashMap::new(),
            img_cache: HashMap::new(),
            alpha_stack: Vec::new(),
            current_alpha: 1.0,
            capture: false,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32, scale: f64) {
        let w = w.max(1);
        let h = h.max(1);
        self.viewport_size = (w, h);
        self.scale = scale;
        let _ = self.layers.resize_layer(self.layer_id, &self.gpu, w, h);
    }

    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    pub fn size(&self) -> Size {
        let (w, h) = self.viewport_size;
        Size::new(w as f64 / self.scale, h as f64 / self.scale)
    }

    /// Flush the batch renderer to the currently bound render target.
    fn flush_batch(&mut self) {
        let _ = self.batch.flush(&self.gpu.device, &self.gpu.context, self.viewport_size);
    }

    // ── Brush helpers ────────────────────────────────────────────

    fn brush_to_color(&self, brush: &BrushRef<'_>) -> [f32; 4] {
        match brush {
            BrushRef::Solid(color) => {
                let [r, g, b, a] = color.to_rgba8().to_u8_array();
                [
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    (a as f32 / 255.0) * self.current_alpha,
                ]
            }
            // TODO: gradient and image brushes
            _ => [0.0, 0.0, 0.0, self.current_alpha],
        }
    }

    // ── Swash glyph rasterization ────────────────────────────────

    fn rasterize_glyph_swash(
        &mut self,
        key: &SwashGlyphKey,
        font_ref: &FontRef<'_>,
        font_size: f32,
        normalized_coords: &[i16],
        offset_x: f32,
        offset_y: f32,
        skew: Option<f32>,
    ) -> Result<()> {
        let image = {
            let mut scaler = self.scale_context
                .builder(*font_ref)
                .size(font_size)
                .hint(true)
                .normalized_coords(normalized_coords)
                .build();

            let mut render = Render::new(&[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ]);
            render
                .format(Format::Alpha)
                .offset(swash::zeno::Vector::new(offset_x.fract(), offset_y.fract()));
            if let Some(angle) = skew {
                render.transform(Some(swash::zeno::Transform::skew(
                    swash::zeno::Angle::from_degrees(angle),
                    swash::zeno::Angle::ZERO,
                )));
            }
            render.render(&mut scaler, key.glyph_id)
        };

        let Some(image) = image else {
            // Empty glyph (e.g. space)
            self.glyph_cache.insert(*key, CachedGlyph {
                uv: [0.0; 4],
                offset: (0, 0),
                size: (0, 0),
            });
            return Ok(());
        };

        let w = image.placement.width;
        let h = image.placement.height;

        if w == 0 || h == 0 {
            self.glyph_cache.insert(*key, CachedGlyph {
                uv: [0.0; 4],
                offset: (0, 0),
                size: (0, 0),
            });
            return Ok(());
        }

        // Convert to alpha mask (R8)
        let alpha_buf: Vec<u8> = match image.content {
            swash::scale::image::Content::Mask => image.data,
            swash::scale::image::Content::Color => {
                // RGBA → extract alpha channel
                image.data.chunks(4).map(|c| c[3]).collect()
            }
            _ => {
                self.glyph_cache.insert(*key, CachedGlyph {
                    uv: [0.0; 4],
                    offset: (0, 0),
                    size: (0, 0),
                });
                return Ok(());
            }
        };

        // Allocate in atlas (clear and retry if full)
        let alloc = match self.atlas.allocate(w, h) {
            Some(a) => a,
            None => {
                self.atlas.clear(&self.gpu.device)?;
                self.glyph_cache.clear();
                self.atlas.allocate(w, h)
                    .ok_or_else(|| Error::new(HRESULT(-1), "glyph too large for atlas"))?
            }
        };

        self.atlas.upload_glyph(&self.gpu.context, &alloc, &alpha_buf, w, h);

        let uv = self.atlas.uv_rect(&alloc, w, h);
        self.glyph_cache.insert(*key, CachedGlyph {
            uv,
            offset: (image.placement.left, image.placement.top),
            size: (w, h),
        });

        Ok(())
    }
}


// ── Renderer trait implementation ────────────────────────────────────────

impl Renderer for WinRenderer {
    fn begin(&mut self, capture: bool) {
        self.capture = capture;
        self.batch.begin();
        self.alpha_stack.clear();
        self.current_alpha = 1.0;

        // Bind the composition layer's render target
        let layer = self.layers.layer(self.layer_id);
        if let Some(rtv) = layer.rtv() {
            let (w, h) = self.viewport_size;
            unsafe {
                self.gpu.context.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
                self.gpu.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: w as f32,
                    Height: h as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                }]));
                self.gpu.context.ClearRenderTargetView(rtv, &[0.0, 0.0, 0.0, 0.0]);
            }
        }
    }

    fn set_transform(&mut self, transform: Affine) {
        self.batch.set_transform(transform);
    }

    fn set_z_index(&mut self, _z_index: i32) {
        // Ignored — we use painter's order like tiny-skia.
    }

    fn clip(&mut self, shape: &impl Shape) {
        let bbox = shape.bounding_box();
        self.batch.push_clip_rect(bbox, self.viewport_size);
    }

    fn clear_clip(&mut self) {
        self.batch.pop_clip_rect();
    }

    fn fill<'b>(&mut self, path: &impl Shape, brush: impl Into<BrushRef<'b>>, _blur_radius: f64) {
        let brush: BrushRef<'b> = brush.into();
        let color = self.brush_to_color(&brush);

        // Try known shape types first (GPU SDF path)
        if let Some(rect) = path.as_rect() {
            self.batch.push_rounded_rect(rect, RoundedRectRadii::default(), color, 0.0);
            return;
        }
        if let Some(rrect) = path.as_rounded_rect() {
            self.batch.push_rounded_rect(rrect.rect(), rrect.radii(), color, 0.0);
            return;
        }
        if let Some(circle) = path.as_circle() {
            self.batch.push_circle(circle.center, circle.radius as f32, color, 0.0);
            return;
        }
        if let Some(line) = path.as_line() {
            self.batch.push_line(line.p0, line.p1, 1.0, color);
            return;
        }

        // Fallback: CPU rasterize with tiny-skia, upload as texture
        self.fill_path_fallback(path, color);
    }

    fn stroke<'b, 's>(
        &mut self,
        shape: &impl Shape,
        brush: impl Into<BrushRef<'b>>,
        stroke: &'s peniko::kurbo::Stroke,
    ) {
        let brush: BrushRef<'b> = brush.into();
        let color = self.brush_to_color(&brush);
        let width = stroke.width as f32;

        // Try known shape types (GPU SDF stroke)
        if let Some(rect) = shape.as_rect() {
            self.batch.push_rounded_rect(rect, RoundedRectRadii::default(), color, width);
            return;
        }
        if let Some(rrect) = shape.as_rounded_rect() {
            self.batch.push_rounded_rect(rrect.rect(), rrect.radii(), color, width);
            return;
        }
        if let Some(circle) = shape.as_circle() {
            self.batch.push_circle(circle.center, circle.radius as f32, color, width);
            return;
        }
        if let Some(line) = shape.as_line() {
            self.batch.push_line(line.p0, line.p1, width, color);
            return;
        }

        // Fallback: stroke the path via tiny-skia
        self.stroke_path_fallback(shape, color, stroke);
    }

    fn push_layer(
        &mut self,
        _blend: impl Into<BlendMode>,
        alpha: f32,
        _transform: Affine,
        clip: &impl Shape,
    ) {
        // Simplified: just track alpha and clip
        self.alpha_stack.push(self.current_alpha);
        self.current_alpha *= alpha;
        let bbox = clip.bounding_box();
        self.batch.push_clip_rect(bbox, self.viewport_size);
    }

    fn pop_layer(&mut self) {
        if let Some(prev_alpha) = self.alpha_stack.pop() {
            self.current_alpha = prev_alpha;
        }
        self.batch.pop_clip_rect();
    }

    fn draw_glyphs<'a>(
        &mut self,
        origin: Point,
        props: &GlyphRunProps<'a>,
        glyphs: impl Iterator<Item = Glyph> + 'a,
    ) {
        let font = &props.font;
        let font_data = font.data.data();
        let font_ref = match FontRef::from_index(font_data.as_ref(), font.index as usize) {
            Some(f) => f,
            None => return,
        };
        let font_blob_id = font.data.id();
        let font_size = props.font_size;

        let brush_color = self.brush_to_color(&props.brush);

        let skew = props
            .glyph_transform
            .map(|t| t.as_coeffs()[2].atan().to_degrees() as f32);

        // Apply the glyph run's transform offset
        let coeffs = props.transform.as_coeffs();
        let run_offset = Point::new(coeffs[4], coeffs[5]);
        let base = origin + run_offset.to_vec2();

        for glyph in glyphs {
            let glyph_x = base.x as f32 + glyph.x;
            let glyph_y = base.y as f32 + glyph.y;

            let (cache_key, snapped_x, snapped_y) = SwashGlyphKey::new(
                font_blob_id,
                font.index,
                glyph.id as u16,
                font_size,
                glyph_x,
                glyph_y,
            );

            if !self.glyph_cache.contains_key(&cache_key) {
                let _ = self.rasterize_glyph_swash(
                    &cache_key,
                    &font_ref,
                    font_size,
                    props.normalized_coords,
                    snapped_x,
                    snapped_y,
                    skew,
                );
            }

            if let Some(cached) = self.glyph_cache.get(&cache_key) {
                if cached.size.0 > 0 && cached.size.1 > 0 {
                    let qx = snapped_x.floor() + cached.offset.0 as f32;
                    let qy = snapped_y.floor() - cached.offset.1 as f32;

                    self.batch.push_text_quad(
                        Rect::new(
                            qx as f64,
                            qy as f64,
                            (qx + cached.size.0 as f32) as f64,
                            (qy + cached.size.1 as f32) as f64,
                        ),
                        [cached.uv[0], cached.uv[1]],
                        [cached.uv[2], cached.uv[3]],
                        brush_color,
                    );
                }
            }
        }

        // Bind the atlas for the text pipeline
        self.batch.set_text_atlas(
            &self.atlas.srv,
            &self.atlas.sampler,
            &self.correction_cbuffer,
        );
    }

    fn draw_svg<'b>(
        &mut self,
        svg: floem_renderer::Svg<'b>,
        rect: Rect,
        brush: Option<impl Into<BrushRef<'b>>>,
    ) {
        let hash_key = svg.hash.to_vec();
        let w = rect.width().max(1.0) as u32;
        let h = rect.height().max(1.0) as u32;

        // Cache lookup
        if !self.svg_cache.contains_key(&hash_key) {
            let size = svg.tree.size();
            let sx = w as f32 / size.width();
            let sy = h as f32 / size.height();
            let transform = Transform::from_scale(sx, sy);

            if let Some(mut pixmap) = Pixmap::new(w, h) {
                render(svg.tree, transform, &mut pixmap.as_mut());
                if let Ok(tex) = GpuTexture::from_rgba(&self.gpu.device, w, h, pixmap.data()) {
                    let key = self.batch.register_image(&tex);
                    self.svg_cache.insert(hash_key.clone(), CachedImage {
                        texture_key: key,
                        gpu_texture: tex,
                    });
                }
            }
        }

        if let Some(cached) = self.svg_cache.get(&hash_key) {
            let tint = if let Some(b) = brush {
                let br: BrushRef<'b> = b.into();
                self.brush_to_color(&br)
            } else {
                [1.0, 1.0, 1.0, self.current_alpha]
            };
            self.batch.push_image_quad(
                rect,
                [0.0, 0.0],
                [1.0, 1.0],
                tint,
                cached.texture_key,
            );
        }
    }

    fn draw_img(&mut self, img: floem_renderer::Img<'_>, rect: Rect) {
        let hash_key = img.hash.to_vec();

        if !self.img_cache.contains_key(&hash_key) {
            let image_data = peniko::ImageData {
                data: img.img.image.data.clone(),
                format: img.img.image.format,
                alpha_type: img.img.image.alpha_type,
                width: img.img.image.width,
                height: img.img.image.height,
            };
            if let Ok(tex) = GpuTexture::from_image(&self.gpu.device, &image_data) {
                let key = self.batch.register_image(&tex);
                self.img_cache.insert(hash_key.clone(), CachedImage {
                    texture_key: key,
                    gpu_texture: tex,
                });
            }
        }

        if let Some(cached) = self.img_cache.get(&hash_key) {
            self.batch.push_image_quad(
                rect,
                [0.0, 0.0],
                [1.0, 1.0],
                [1.0, 1.0, 1.0, self.current_alpha],
                cached.texture_key,
            );
        }
    }

    fn finish(&mut self) -> Option<peniko::ImageBrush> {
        self.flush_batch();

        // Present the composition layer
        self.layers.layer_mut(self.layer_id).mark_dirty();
        let now = win_renderer::util::now_seconds();
        let _ = self.layers.present_layer(
            self.layer_id, &self.gpu.device, 1, 0, now,
        );
        let _ = self.layers.commit();

        if self.capture {
            // TODO: Read back render target as ImageBrush for capture mode
            None
        } else {
            None
        }
    }

    fn debug_info(&self) -> String {
        "name: win-renderer\nbackend: Direct3D 11".into()
    }
}


// ── Fallback path rendering via tiny-skia ────────────────────────────────

impl WinRenderer {
    fn fill_path_fallback(&mut self, path: &impl Shape, color: [f32; 4]) {
        let bbox = path.bounding_box();
        let w = bbox.width().ceil().max(1.0) as u32;
        let h = bbox.height().ceil().max(1.0) as u32;

        let Some(mut pixmap) = Pixmap::new(w, h) else { return };

        let mut pb = PathBuilder::new();
        for el in path.path_elements(0.1) {
            match el {
                kurbo::PathEl::MoveTo(p) => pb.move_to(
                    (p.x - bbox.x0) as f32,
                    (p.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::LineTo(p) => pb.line_to(
                    (p.x - bbox.x0) as f32,
                    (p.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::QuadTo(p1, p2) => pb.quad_to(
                    (p1.x - bbox.x0) as f32,
                    (p1.y - bbox.y0) as f32,
                    (p2.x - bbox.x0) as f32,
                    (p2.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::CurveTo(p1, p2, p3) => pb.cubic_to(
                    (p1.x - bbox.x0) as f32,
                    (p1.y - bbox.y0) as f32,
                    (p2.x - bbox.x0) as f32,
                    (p2.y - bbox.y0) as f32,
                    (p3.x - bbox.x0) as f32,
                    (p3.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::ClosePath => pb.close(),
            }
        }

        let Some(ts_path) = pb.finish() else { return };

        let c = to_tiny_skia_color(color);
        let paint = tiny_skia::Paint {
            shader: tiny_skia::Shader::SolidColor(c),
            anti_alias: true,
            ..Default::default()
        };

        pixmap.fill_path(
            &ts_path,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        self.upload_pixmap_as_quad(&pixmap, bbox);
    }

    fn stroke_path_fallback(
        &mut self,
        shape: &impl Shape,
        color: [f32; 4],
        stroke: &peniko::kurbo::Stroke,
    ) {
        // Expand bounding box by stroke width for the rasterization area
        let half_w = stroke.width * 0.5;
        let bbox = shape.bounding_box().inflate(half_w, half_w);
        let w = bbox.width().ceil().max(1.0) as u32;
        let h = bbox.height().ceil().max(1.0) as u32;

        let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else { return };

        let mut pb = tiny_skia::PathBuilder::new();
        for el in shape.path_elements(0.1) {
            match el {
                kurbo::PathEl::MoveTo(p) => pb.move_to(
                    (p.x - bbox.x0) as f32,
                    (p.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::LineTo(p) => pb.line_to(
                    (p.x - bbox.x0) as f32,
                    (p.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::QuadTo(p1, p2) => pb.quad_to(
                    (p1.x - bbox.x0) as f32,
                    (p1.y - bbox.y0) as f32,
                    (p2.x - bbox.x0) as f32,
                    (p2.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::CurveTo(p1, p2, p3) => pb.cubic_to(
                    (p1.x - bbox.x0) as f32,
                    (p1.y - bbox.y0) as f32,
                    (p2.x - bbox.x0) as f32,
                    (p2.y - bbox.y0) as f32,
                    (p3.x - bbox.x0) as f32,
                    (p3.y - bbox.y0) as f32,
                ),
                kurbo::PathEl::ClosePath => pb.close(),
            }
        }

        let Some(ts_path) = pb.finish() else { return };

        let c = to_tiny_skia_color(color);
        let paint = tiny_skia::Paint {
            shader: tiny_skia::Shader::SolidColor(c),
            anti_alias: true,
            ..Default::default()
        };

        let ts_stroke = tiny_skia::Stroke {
            width: stroke.width as f32,
            ..Default::default()
        };

        pixmap.stroke_path(
            &ts_path,
            &paint,
            &ts_stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        self.upload_pixmap_as_quad(&pixmap, bbox);
    }

    fn upload_pixmap_as_quad(&mut self, pixmap: &tiny_skia::Pixmap, bbox: Rect) {
        let Ok(tex) = GpuTexture::from_rgba(&self.gpu.device, pixmap.width(), pixmap.height(), pixmap.data()) else {
            return;
        };
        let key = self.batch.register_image(&tex);
        self.batch.push_image_quad(
            bbox,
            [0.0, 0.0],
            [1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            key,
        );
    }
}


// ── Helpers ──────────────────────────────────────────────────────────────

fn to_tiny_skia_color(c: [f32; 4]) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c[0], c[1], c[2], c[3])
        .unwrap_or(tiny_skia::Color::BLACK)
}
