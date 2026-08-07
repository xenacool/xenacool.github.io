use pystral_core::log::{PropertyValue, EntityState};
use pystral_core::domain::Material;
use pystral_core::render::{RenderError, ERROR_MODE_ENABLED};
use std::sync::atomic::Ordering;

use crate::WorkerInput;

pub type RenderResult<T> = Result<T, RenderError<T>>;

pub trait RenderResultExt<T> {
    fn log_fallback(self, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>) -> T;
}

impl<T> RenderResultExt<T> for RenderResult<T> {
    fn log_fallback(self, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                let _ = worker_tx.unbounded_send(WorkerInput::LogError(e.message));
                e.fallback
            }
        }
    }
}

pub trait EntityExt {
    fn get_float(&self, key: &str, default: f32) -> RenderResult<f32>;
    fn get_material(&self, materials: &std::collections::HashMap<String, Material>) -> RenderResult<Material>;
    fn get_hex_map(&self) -> RenderResult<pystral_core::domain::HexMap>;
    fn get_lighting(&self) -> RenderResult<pystral_core::domain::LightingConfig>;
    fn get_collision(&self) -> RenderResult<Option<pystral_core::domain::Shape3D>>;
}

pub fn interpolate_property(from: &PropertyValue, to: &PropertyValue, t: f32) -> PropertyValue {
    match (from, to) {
        (PropertyValue::Float(f1), PropertyValue::Float(f2)) => PropertyValue::Float(f1 + (f2 - f1) * t),
        (PropertyValue::Vec3(v1), PropertyValue::Vec3(v2)) => PropertyValue::Vec3(*v1 + (*v2 - *v1) * t),
        (PropertyValue::Color(c1), PropertyValue::Color(c2)) => {
            let mut c = [0.0; 3];
            for i in 0..3 { c[i] = c1[i] + (c2[i] - c1[i]) * t; }
            PropertyValue::Color(c)
        }
        _ => to.clone(),
    }
}

impl EntityExt for EntityState {
    fn get_float(&self, key: &str, default: f32) -> RenderResult<f32> {
        if let Some(PropertyValue::Float(v)) = self.properties.get(key) {
            if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                Err(RenderError::new(format!("Property {} found but ERROR_MODE_ENABLED is set", key), *v))
            } else {
                Ok(*v)
            }
        } else {
            Err(RenderError::new(format!("Property {} not found", key), default))
        }
    }


    fn get_material(&self, materials: &std::collections::HashMap<String, Material>) -> RenderResult<Material> {
        let default = Material {
            color: [1.0, 1.0, 1.0],
            roughness: 0.5,
            metalness: 0.0,
            emissive: 0.0,
        };
        if let Some(prop) = self.properties.get("material") {
            match prop {
                PropertyValue::Material(m) => {
                    if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                        Err(RenderError::new("Property material found but ERROR_MODE_ENABLED is set", m.clone()))
                    } else {
                        Ok(m.clone())
                    }
                }
                PropertyValue::String(name) => {
                    if let Some(m) = materials.get(name) {
                        if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                            Err(RenderError::new(format!("Material {} found but ERROR_MODE_ENABLED is set", name), m.clone()))
                        } else {
                            Ok(m.clone())
                        }
                    } else {
                        Err(RenderError::new(format!("Material {} not found in world", name), default))
                    }
                }
                _ => Err(RenderError::new("Property material is not a Material or String", default)),
            }
        } else {
            Err(RenderError::new("Property material not found", default))
        }
    }

    fn get_hex_map(&self) -> RenderResult<pystral_core::domain::HexMap> {
        let key = "map";
        if let Some(PropertyValue::HexMap(m)) = self.properties.get(key) {
            if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                Err(RenderError::new("Property map found but ERROR_MODE_ENABLED is set", m.clone()))
            } else {
                Ok(m.clone())
            }
        } else {
            Err(RenderError::new("Property map not found", pystral_core::domain::HexMap::new()))
        }
    }

    fn get_lighting(&self) -> RenderResult<pystral_core::domain::LightingConfig> {
        let key = "lighting";
        let default = pystral_core::domain::LightingConfig::default();
        if let Some(PropertyValue::Lighting(l)) = self.properties.get(key) {
            if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                Err(RenderError::new("Property lighting found but ERROR_MODE_ENABLED is set", l.clone()))
            } else {
                Ok(l.clone())
            }
        } else {
            Err(RenderError::new("Property lighting not found", default))
        }
    }

    fn get_collision(&self) -> RenderResult<Option<pystral_core::domain::Shape3D>> {
        let key = "collision";
        if let Some(PropertyValue::Shape3D(s)) = self.properties.get(key) {
            if ERROR_MODE_ENABLED.load(Ordering::Relaxed) {
                Err(RenderError::new("Property collision found but ERROR_MODE_ENABLED is set", Some(s.clone())))
            } else {
                Ok(Some(s.clone()))
            }
        } else {
            Ok(None) // It's fine for an entity to not have collision
        }
    }
}
