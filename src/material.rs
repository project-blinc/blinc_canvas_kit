//! Ergonomic PBR material builder for 3D scenes.
//!
//! Wraps `blinc_core::Material` with a fluent builder API so common
//! materials can be created in one expression:
//!
//! ```ignore
//! use blinc_canvas_kit::prelude::*;
//! use blinc_core::Color;
//!
//! let gold = MaterialBuilder::standard()
//!     .color(Color::from_hex(0xFFD700))
//!     .metallic(1.0)
//!     .roughness(0.3);
//!
//! let rubber = MaterialBuilder::standard()
//!     .color(Color::from_hex(0x222222))
//!     .metallic(0.0)
//!     .roughness(0.9);
//! ```

use blinc_core::draw::{AlphaMode, Material, TextureData};
use blinc_core::Color;

/// Fluent builder for PBR materials. Call `.build()` to get a
/// `Material`, or pass directly to `SceneKit3D::add` which accepts
/// `Into<Material>`.
#[derive(Clone, Debug)]
pub struct MaterialBuilder {
    inner: Material,
}

impl MaterialBuilder {
    /// Standard PBR material with sensible defaults: white, non-metallic,
    /// medium roughness.
    pub fn standard() -> Self {
        Self {
            inner: Material {
                base_color: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                ..Default::default()
            },
        }
    }

    /// Unlit material — ignores all lighting, renders flat color. Good
    /// for UI overlays, wireframes, debug visualizations.
    pub fn unlit() -> Self {
        Self {
            inner: Material {
                unlit: true,
                ..Default::default()
            },
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.inner.base_color = [color.r, color.g, color.b, color.a];
        self
    }

    pub fn metallic(mut self, metallic: f32) -> Self {
        self.inner.metallic = metallic;
        self
    }

    pub fn roughness(mut self, roughness: f32) -> Self {
        self.inner.roughness = roughness;
        self
    }

    pub fn emissive(mut self, r: f32, g: f32, b: f32) -> Self {
        self.inner.emissive = [r, g, b];
        self
    }

    pub fn emissive_color(mut self, color: Color) -> Self {
        self.inner.emissive = [color.r, color.g, color.b];
        self
    }

    pub fn base_color_texture(mut self, texture: TextureData) -> Self {
        self.inner.base_color_texture = Some(texture);
        self
    }

    pub fn normal_map(mut self, texture: TextureData) -> Self {
        self.inner.normal_map = Some(texture);
        self
    }

    pub fn normal_scale(mut self, scale: f32) -> Self {
        self.inner.normal_scale = scale;
        self
    }

    pub fn metallic_roughness_texture(mut self, texture: TextureData) -> Self {
        self.inner.metallic_roughness_texture = Some(texture);
        self
    }

    pub fn emissive_texture(mut self, texture: TextureData) -> Self {
        self.inner.emissive_texture = Some(texture);
        self
    }

    pub fn occlusion_texture(mut self, texture: TextureData, strength: f32) -> Self {
        self.inner.occlusion_texture = Some(texture);
        self.inner.occlusion_strength = strength;
        self
    }

    pub fn alpha_blend(mut self) -> Self {
        self.inner.alpha_mode = AlphaMode::Blend;
        self
    }

    pub fn alpha_mask(mut self) -> Self {
        self.inner.alpha_mode = AlphaMode::Mask;
        self
    }

    pub fn double_sided(mut self) -> Self {
        self.inner.casts_shadows = false;
        self
    }

    pub fn build(self) -> Material {
        self.inner
    }
}

impl From<MaterialBuilder> for Material {
    fn from(builder: MaterialBuilder) -> Self {
        builder.inner
    }
}
