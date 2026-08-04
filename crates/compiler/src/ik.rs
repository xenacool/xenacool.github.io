pub use constraint_solver::{Compiler, Exp, NewtonRaphsonSolver};
use std::collections::HashMap;

/// A point in 3D space for the IK solver.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A request to solve an IK problem for a specific time or frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IkRequest {
    pub rig_id: String,
    pub targets: HashMap<String, Vec3>,
    pub initial_guesses: HashMap<String, Vec3>,
}

/// The result of an IK solver pass.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IkResponse {
    pub joints: HashMap<String, Vec3>,
}

/// Defines a jointed structure that can be solved.
pub struct Rig {
    pub solver: NewtonRaphsonSolver,
    pub variable_names: Vec<String>,
    pub target_names: Vec<String>,
}

#[derive(Default)]
pub struct IkSystem {
    pub rigs: HashMap<String, Rig>,
}

impl IkSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rig(&mut self, id: &str, rig: Rig) {
        self.rigs.insert(id.to_string(), rig);
    }

    pub fn solve(&self, request: &IkRequest) -> Result<IkResponse, String> {
        let rig = self
            .rigs
            .get(&request.rig_id)
            .ok_or_else(|| format!("Rig {} not found", request.rig_id))?;

        let mut values = HashMap::new();

        // Fill targets
        for target_name in &rig.target_names {
            if let Some(pos) = request.targets.get(target_name) {
                values.insert(format!("{}_x", target_name), pos.x as f64);
                values.insert(format!("{}_y", target_name), pos.y as f64);
                values.insert(format!("{}_z", target_name), pos.z as f64);
            } else {
                return Err(format!("Missing target: {}", target_name));
            }
        }

        // Fill initial guesses for variables
        for var_name in &rig.variable_names {
            if let Some(pos) = request.initial_guesses.get(var_name) {
                values.insert(format!("{}_x", var_name), pos.x as f64);
                values.insert(format!("{}_y", var_name), pos.y as f64);
                values.insert(format!("{}_z", var_name), pos.z as f64);
            } else {
                // Default to a small bias to prevent crossing if no guess provided
                let bias_x = if var_name.starts_with('l') {
                    -0.3
                } else if var_name.starts_with('r') {
                    0.3
                } else {
                    0.0
                };
                values.insert(format!("{}_x", var_name), bias_x);
                values.insert(format!("{}_y", var_name), 0.0);
                values.insert(format!("{}_z", var_name), 0.5); // Start mid-height
            }
        }

        let result = rig
            .solver
            .solve(values)
            .map_err(|e| format!("IK Solver error: {:?}", e))?;

        let mut joints = HashMap::new();
        for var_name in &rig.variable_names {
            let x = *result
                .values
                .get(&format!("{}_x", var_name))
                .unwrap_or(&0.0);
            let y = *result
                .values
                .get(&format!("{}_y", var_name))
                .unwrap_or(&0.0);
            let z = *result
                .values
                .get(&format!("{}_z", var_name))
                .unwrap_or(&0.0);
            joints.insert(
                var_name.clone(),
                Vec3 {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32,
                },
            );
        }

        Ok(IkResponse { joints })
    }
}

pub fn length_eq(ax: &Exp, ay: &Exp, az: &Exp, bx: &Exp, by: &Exp, bz: &Exp, length: f64) -> Exp {
    let dx = Exp::sub(ax.clone(), bx.clone());
    let dy = Exp::sub(ay.clone(), by.clone());
    let dz = Exp::sub(az.clone(), bz.clone());
    let dist_sq = Exp::add(
        Exp::add(Exp::power(dx, 2.0), Exp::power(dy, 2.0)),
        Exp::power(dz, 2.0),
    );
    Exp::sub(dist_sq, Exp::val(length * length))
}
