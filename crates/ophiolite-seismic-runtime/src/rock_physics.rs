use std::collections::BTreeMap;

use ophiolite_seismic::{
    AvoInterceptGradientAttributeMethod, AvoInterceptGradientAttributeRequest,
    AvoInterceptGradientAttributeResponse, RockPhysicsAttributeMethod, RockPhysicsAttributeRequest,
    RockPhysicsAttributeResponse,
};
use rayon::prelude::*;

use crate::error::SeismicStoreError;

#[derive(Debug, Clone, Copy)]
struct ImpedanceReferenceTerms {
    vp0_m_per_s: f64,
    vs0_m_per_s: f64,
    density0_g_cc: f64,
    velocity_ratio_k: f64,
}

pub fn rock_physics_attribute(
    request: RockPhysicsAttributeRequest,
) -> Result<RockPhysicsAttributeResponse, SeismicStoreError> {
    let sample_count = validate_sample_shape(&request.sample_shape)?;
    let vp = optional_samples(request.vp_m_per_s.as_deref(), "vp_m_per_s", sample_count)?;
    let vs = optional_samples(request.vs_m_per_s.as_deref(), "vs_m_per_s", sample_count)?;
    let density = optional_samples(
        request.density_g_cc.as_deref(),
        "density_g_cc",
        sample_count,
    )?;

    let semantic_parameters = match request.method {
        RockPhysicsAttributeMethod::ElasticImpedance => {
            let angle_deg = validate_incident_angle(request.incident_angle_deg)?;
            let terms = impedance_reference_terms(
                required_samples(vp, "vp_m_per_s")?,
                required_samples(vs, "vs_m_per_s")?,
                required_samples(density, "density_g_cc")?,
            )?;
            impedance_semantic_parameters("incident_angle_deg", angle_deg, terms)
        }
        RockPhysicsAttributeMethod::ExtendedElasticImpedance => {
            let chi_deg = validate_chi_angle(request.chi_angle_deg)?;
            let terms = impedance_reference_terms(
                required_samples(vp, "vp_m_per_s")?,
                required_samples(vs, "vs_m_per_s")?,
                required_samples(density, "density_g_cc")?,
            )?;
            impedance_semantic_parameters("chi_angle_deg", chi_deg, terms)
        }
        _ => BTreeMap::new(),
    };

    let values = (0..sample_count)
        .into_par_iter()
        .map(|sample_index| {
            let value = match request.method {
                RockPhysicsAttributeMethod::AcousticImpedance => acoustic_impedance_sample(
                    sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                    sample(required_samples(density, "density_g_cc")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::ShearImpedance => shear_impedance_sample(
                    sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                    sample(required_samples(density, "density_g_cc")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::LambdaRho => lambda_rho_sample(
                    sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                    sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                    sample(required_samples(density, "density_g_cc")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::MuRho => mu_rho_sample(
                    sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                    sample(required_samples(density, "density_g_cc")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::VpVsRatio => vp_vs_ratio_sample(
                    sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                    sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::PoissonsRatio => poissons_ratio_sample(
                    sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                    sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                )?,
                RockPhysicsAttributeMethod::ElasticImpedance => {
                    let angle_deg = semantic_parameters
                        .get("incident_angle_deg")
                        .copied()
                        .expect("incident angle is present");
                    normalized_elastic_impedance_sample(
                        sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                        sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                        sample(required_samples(density, "density_g_cc")?, sample_index),
                        angle_deg,
                        impedance_reference_terms_from_map(&semantic_parameters)?,
                    )?
                }
                RockPhysicsAttributeMethod::ExtendedElasticImpedance => {
                    let chi_deg = semantic_parameters
                        .get("chi_angle_deg")
                        .copied()
                        .expect("chi angle is present");
                    extended_elastic_impedance_sample(
                        sample(required_samples(vp, "vp_m_per_s")?, sample_index),
                        sample(required_samples(vs, "vs_m_per_s")?, sample_index),
                        sample(required_samples(density, "density_g_cc")?, sample_index),
                        chi_deg,
                        impedance_reference_terms_from_map(&semantic_parameters)?,
                    )?
                }
            };
            Ok(value as f32)
        })
        .collect::<Result<Vec<_>, SeismicStoreError>>()?;

    Ok(RockPhysicsAttributeResponse {
        schema_version: request.schema_version,
        method: request.method,
        sample_shape: request.sample_shape,
        unit: unit_for_method(request.method),
        values_f32le: f32_vec_to_le_bytes(&values),
        semantic_parameters,
    })
}

pub fn avo_intercept_gradient_attribute(
    request: AvoInterceptGradientAttributeRequest,
) -> Result<AvoInterceptGradientAttributeResponse, SeismicStoreError> {
    let sample_count = validate_sample_shape(&request.sample_shape)?;
    validate_exact_len("intercept", &request.intercept, sample_count)?;
    validate_exact_len("gradient", &request.gradient, sample_count)?;

    let semantic_parameters = match request.method {
        AvoInterceptGradientAttributeMethod::ChiProjection => BTreeMap::from([(
            "chi_angle_deg".to_string(),
            validate_chi_angle(request.chi_angle_deg)?,
        )]),
        AvoInterceptGradientAttributeMethod::FluidFactor => BTreeMap::from([(
            "intercept_scalar".to_string(),
            validate_finite_scalar(request.intercept_scalar, "intercept_scalar")?,
        )]),
    };

    let values = (0..sample_count)
        .into_par_iter()
        .map(|sample_index| {
            let intercept =
                validate_finite_sample(request.intercept[sample_index] as f64, "intercept")?;
            let gradient =
                validate_finite_sample(request.gradient[sample_index] as f64, "gradient")?;
            let value = match request.method {
                AvoInterceptGradientAttributeMethod::ChiProjection => {
                    let chi = semantic_parameters
                        .get("chi_angle_deg")
                        .copied()
                        .expect("chi angle is present")
                        .to_radians();
                    intercept * chi.cos() + gradient * chi.sin()
                }
                AvoInterceptGradientAttributeMethod::FluidFactor => {
                    let intercept_scalar = semantic_parameters
                        .get("intercept_scalar")
                        .copied()
                        .expect("intercept scalar is present");
                    gradient - (intercept_scalar * intercept)
                }
            };
            Ok(value as f32)
        })
        .collect::<Result<Vec<_>, SeismicStoreError>>()?;

    Ok(AvoInterceptGradientAttributeResponse {
        schema_version: request.schema_version,
        method: request.method,
        sample_shape: request.sample_shape,
        unit: None,
        values_f32le: f32_vec_to_le_bytes(&values),
        semantic_parameters,
    })
}

fn unit_for_method(method: RockPhysicsAttributeMethod) -> Option<String> {
    Some(
        match method {
            RockPhysicsAttributeMethod::AcousticImpedance
            | RockPhysicsAttributeMethod::ShearImpedance
            | RockPhysicsAttributeMethod::ElasticImpedance
            | RockPhysicsAttributeMethod::ExtendedElasticImpedance => "(m/s)*(g/cc)",
            RockPhysicsAttributeMethod::LambdaRho | RockPhysicsAttributeMethod::MuRho => "GPa",
            RockPhysicsAttributeMethod::VpVsRatio => "ratio",
            RockPhysicsAttributeMethod::PoissonsRatio => return None,
        }
        .to_string(),
    )
}

fn optional_samples<'a>(
    values: Option<&'a [f32]>,
    name: &str,
    sample_count: usize,
) -> Result<Option<&'a [f32]>, SeismicStoreError> {
    if let Some(values) = values {
        validate_exact_len(name, values, sample_count)?;
        Ok(Some(values))
    } else {
        Ok(None)
    }
}

fn required_samples<'a>(
    values: Option<&'a [f32]>,
    name: &str,
) -> Result<&'a [f32], SeismicStoreError> {
    values
        .ok_or_else(|| SeismicStoreError::Message(format!("'{name}' is required for this method")))
}

fn sample(values: &[f32], index: usize) -> f64 {
    values[index] as f64
}

fn validate_sample_shape(sample_shape: &[usize]) -> Result<usize, SeismicStoreError> {
    if sample_shape.is_empty() {
        return Err(SeismicStoreError::Message(
            "'sample_shape' must contain at least one dimension".to_string(),
        ));
    }
    sample_shape.iter().try_fold(1usize, |acc, dim| {
        if *dim == 0 {
            return Err(SeismicStoreError::Message(
                "'sample_shape' cannot contain zero-length dimensions".to_string(),
            ));
        }
        acc.checked_mul(*dim).ok_or_else(|| {
            SeismicStoreError::Message("'sample_shape' overflows total sample count".to_string())
        })
    })
}

fn validate_exact_len(
    name: &str,
    values: &[f32],
    sample_count: usize,
) -> Result<(), SeismicStoreError> {
    if values.len() != sample_count {
        return Err(SeismicStoreError::Message(format!(
            "'{name}' length {} does not match sample count {sample_count}",
            values.len()
        )));
    }
    Ok(())
}

fn validate_positive_sample(value: f64, name: &str) -> Result<f64, SeismicStoreError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(SeismicStoreError::Message(format!(
            "'{name}' must contain positive finite samples"
        )))
    }
}

fn validate_finite_sample(value: f64, name: &str) -> Result<f64, SeismicStoreError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SeismicStoreError::Message(format!(
            "'{name}' must contain finite samples"
        )))
    }
}

fn validate_finite_scalar(value: Option<f32>, name: &str) -> Result<f64, SeismicStoreError> {
    let value = value.ok_or_else(|| {
        SeismicStoreError::Message(format!("'{name}' is required for this method"))
    })?;
    validate_finite_sample(value as f64, name)
}

fn validate_incident_angle(value: Option<f32>) -> Result<f64, SeismicStoreError> {
    let angle_deg = validate_finite_scalar(value, "incident_angle_deg")?;
    if (0.0..90.0).contains(&angle_deg) {
        Ok(angle_deg)
    } else {
        Err(SeismicStoreError::Message(
            "'incident_angle_deg' must be in [0, 90)".to_string(),
        ))
    }
}

fn validate_chi_angle(value: Option<f32>) -> Result<f64, SeismicStoreError> {
    let chi_deg = validate_finite_scalar(value, "chi_angle_deg")?;
    if (-90.0..=90.0).contains(&chi_deg) {
        Ok(chi_deg)
    } else {
        Err(SeismicStoreError::Message(
            "'chi_angle_deg' must be in [-90, 90]".to_string(),
        ))
    }
}

fn acoustic_impedance_sample(vp_m_per_s: f64, density_g_cc: f64) -> Result<f64, SeismicStoreError> {
    Ok(validate_positive_sample(vp_m_per_s, "vp_m_per_s")?
        * validate_positive_sample(density_g_cc, "density_g_cc")?)
}

fn shear_impedance_sample(vs_m_per_s: f64, density_g_cc: f64) -> Result<f64, SeismicStoreError> {
    Ok(validate_positive_sample(vs_m_per_s, "vs_m_per_s")?
        * validate_positive_sample(density_g_cc, "density_g_cc")?)
}

fn vp_vs_ratio_sample(vp_m_per_s: f64, vs_m_per_s: f64) -> Result<f64, SeismicStoreError> {
    Ok(validate_positive_sample(vp_m_per_s, "vp_m_per_s")?
        / validate_positive_sample(vs_m_per_s, "vs_m_per_s")?)
}

fn poissons_ratio_sample(vp_m_per_s: f64, vs_m_per_s: f64) -> Result<f64, SeismicStoreError> {
    let ratio_sq = (validate_positive_sample(vp_m_per_s, "vp_m_per_s")?
        / validate_positive_sample(vs_m_per_s, "vs_m_per_s")?)
    .powi(2);
    let denominator = 2.0 * (ratio_sq - 1.0);
    if denominator.abs() <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "Poisson's ratio denominator collapses for the provided samples".to_string(),
        ));
    }
    Ok((ratio_sq - 2.0) / denominator)
}

fn lambda_rho_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_g_cc: f64,
) -> Result<f64, SeismicStoreError> {
    let vp_km_per_s = validate_positive_sample(vp_m_per_s, "vp_m_per_s")? / 1000.0;
    let vs_km_per_s = validate_positive_sample(vs_m_per_s, "vs_m_per_s")? / 1000.0;
    let density_g_cc = validate_positive_sample(density_g_cc, "density_g_cc")?;
    Ok(density_g_cc * (vp_km_per_s.powi(2) - (2.0 * vs_km_per_s.powi(2))))
}

fn mu_rho_sample(vs_m_per_s: f64, density_g_cc: f64) -> Result<f64, SeismicStoreError> {
    let vs_km_per_s = validate_positive_sample(vs_m_per_s, "vs_m_per_s")? / 1000.0;
    let density_g_cc = validate_positive_sample(density_g_cc, "density_g_cc")?;
    Ok(density_g_cc * vs_km_per_s.powi(2))
}

fn impedance_reference_terms(
    vp_m_per_s: &[f32],
    vs_m_per_s: &[f32],
    density_g_cc: &[f32],
) -> Result<ImpedanceReferenceTerms, SeismicStoreError> {
    let mut count = 0usize;
    let mut vp_sum = 0.0;
    let mut vs_sum = 0.0;
    let mut density_sum = 0.0;
    let mut ratio_sum = 0.0;
    for ((vp, vs), density) in vp_m_per_s.iter().zip(vs_m_per_s).zip(density_g_cc) {
        let vp = *vp as f64;
        let vs = *vs as f64;
        let density = *density as f64;
        if !vp.is_finite() || !vs.is_finite() || !density.is_finite() {
            continue;
        }
        if vp <= 0.0 || vs <= 0.0 || density <= 0.0 {
            continue;
        }
        vp_sum += vp;
        vs_sum += vs;
        density_sum += density;
        ratio_sum += (vs / vp).powi(2);
        count += 1;
    }
    if count == 0 {
        return Err(SeismicStoreError::Message(
            "elastic impedance methods require at least one positive finite Vp, Vs, and density sample"
                .to_string(),
        ));
    }
    Ok(ImpedanceReferenceTerms {
        vp0_m_per_s: vp_sum / count as f64,
        vs0_m_per_s: vs_sum / count as f64,
        density0_g_cc: density_sum / count as f64,
        velocity_ratio_k: ratio_sum / count as f64,
    })
}

fn impedance_semantic_parameters(
    angle_name: &str,
    angle_value: f64,
    terms: ImpedanceReferenceTerms,
) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (angle_name.to_string(), angle_value),
        (
            "normalization_reference_vp_m_per_s".to_string(),
            terms.vp0_m_per_s,
        ),
        (
            "normalization_reference_vs_m_per_s".to_string(),
            terms.vs0_m_per_s,
        ),
        (
            "normalization_reference_density_g_cc".to_string(),
            terms.density0_g_cc,
        ),
        ("velocity_ratio_k".to_string(), terms.velocity_ratio_k),
    ])
}

fn impedance_reference_terms_from_map(
    semantic_parameters: &BTreeMap<String, f64>,
) -> Result<ImpedanceReferenceTerms, SeismicStoreError> {
    Ok(ImpedanceReferenceTerms {
        vp0_m_per_s: *semantic_parameters
            .get("normalization_reference_vp_m_per_s")
            .ok_or_else(|| {
                SeismicStoreError::Message("missing normalization_reference_vp_m_per_s".to_string())
            })?,
        vs0_m_per_s: *semantic_parameters
            .get("normalization_reference_vs_m_per_s")
            .ok_or_else(|| {
                SeismicStoreError::Message("missing normalization_reference_vs_m_per_s".to_string())
            })?,
        density0_g_cc: *semantic_parameters
            .get("normalization_reference_density_g_cc")
            .ok_or_else(|| {
                SeismicStoreError::Message(
                    "missing normalization_reference_density_g_cc".to_string(),
                )
            })?,
        velocity_ratio_k: *semantic_parameters
            .get("velocity_ratio_k")
            .ok_or_else(|| SeismicStoreError::Message("missing velocity_ratio_k".to_string()))?,
    })
}

fn normalized_elastic_impedance_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_g_cc: f64,
    angle_deg: f64,
    terms: ImpedanceReferenceTerms,
) -> Result<f64, SeismicStoreError> {
    let vp_m_per_s = validate_positive_sample(vp_m_per_s, "vp_m_per_s")?;
    let vs_m_per_s = validate_positive_sample(vs_m_per_s, "vs_m_per_s")?;
    let density_g_cc = validate_positive_sample(density_g_cc, "density_g_cc")?;
    let theta = angle_deg.to_radians();
    let a = 1.0 + theta.tan().powi(2);
    let b = -8.0 * terms.velocity_ratio_k * theta.sin().powi(2);
    let c = 1.0 - 4.0 * terms.velocity_ratio_k * theta.sin().powi(2);
    Ok(vp_m_per_s.powf(a)
        * vs_m_per_s.powf(b)
        * density_g_cc.powf(c)
        * terms.vp0_m_per_s.powf(1.0 - a)
        * terms.vs0_m_per_s.powf(-b)
        * terms.density0_g_cc.powf(1.0 - c))
}

fn extended_elastic_impedance_sample(
    vp_m_per_s: f64,
    vs_m_per_s: f64,
    density_g_cc: f64,
    chi_deg: f64,
    terms: ImpedanceReferenceTerms,
) -> Result<f64, SeismicStoreError> {
    let vp_m_per_s = validate_positive_sample(vp_m_per_s, "vp_m_per_s")?;
    let vs_m_per_s = validate_positive_sample(vs_m_per_s, "vs_m_per_s")?;
    let density_g_cc = validate_positive_sample(density_g_cc, "density_g_cc")?;
    let chi = chi_deg.to_radians();
    let p = chi.cos() + chi.sin();
    let q = -8.0 * terms.velocity_ratio_k * chi.sin();
    let r = chi.cos() - 4.0 * terms.velocity_ratio_k * chi.sin();
    Ok(terms.vp0_m_per_s
        * terms.density0_g_cc
        * (vp_m_per_s / terms.vp0_m_per_s).powf(p)
        * (vs_m_per_s / terms.vs0_m_per_s).powf(q)
        * (density_g_cc / terms.density0_g_cc).powf(r))
}

fn f32_vec_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_f32le(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn assert_close(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= tolerance,
                "sample {index}: expected {expected}, found {actual}"
            );
        }
    }

    #[test]
    fn rock_physics_attribute_matches_reference_values() {
        let request = RockPhysicsAttributeRequest {
            schema_version: 2,
            method: RockPhysicsAttributeMethod::ElasticImpedance,
            sample_shape: vec![4],
            vp_m_per_s: Some(vec![2600.0, 2800.0, 3000.0, 3100.0]),
            vs_m_per_s: Some(vec![1200.0, 1400.0, 1600.0, 1700.0]),
            density_g_cc: Some(vec![2.15, 2.22, 2.28, 2.35]),
            incident_angle_deg: Some(30.0),
            chi_angle_deg: None,
        };
        let response = rock_physics_attribute(request).unwrap();
        assert_eq!(
            response.method,
            RockPhysicsAttributeMethod::ElasticImpedance
        );
        assert_close(
            &decode_f32le(&response.values_f32le),
            &[5151.158, 5666.6797, 6171.5015, 6649.225],
            1.0e-3,
        );
        assert_eq!(
            response.semantic_parameters.get("incident_angle_deg"),
            Some(&30.0)
        );

        let eei_response = rock_physics_attribute(RockPhysicsAttributeRequest {
            schema_version: 2,
            method: RockPhysicsAttributeMethod::ExtendedElasticImpedance,
            sample_shape: vec![4],
            vp_m_per_s: Some(vec![2600.0, 2800.0, 3000.0, 3100.0]),
            vs_m_per_s: Some(vec![1200.0, 1400.0, 1600.0, 1700.0]),
            density_g_cc: Some(vec![2.15, 2.22, 2.28, 2.35]),
            incident_angle_deg: None,
            chi_angle_deg: Some(20.0),
        })
        .unwrap();
        assert_close(
            &decode_f32le(&eei_response.values_f32le),
            &[5496.696, 5763.6777, 6022.081, 6341.1406],
            1.0e-3,
        );
        assert_eq!(
            eei_response.semantic_parameters.get("chi_angle_deg"),
            Some(&20.0)
        );
    }

    #[test]
    fn rock_physics_attribute_preserves_multidimensional_packing() {
        let response = rock_physics_attribute(RockPhysicsAttributeRequest {
            schema_version: 2,
            method: RockPhysicsAttributeMethod::AcousticImpedance,
            sample_shape: vec![2, 2],
            vp_m_per_s: Some(vec![2000.0, 2100.0, 2200.0, 2300.0]),
            vs_m_per_s: None,
            density_g_cc: Some(vec![2.0, 2.1, 2.2, 2.3]),
            incident_angle_deg: None,
            chi_angle_deg: None,
        })
        .unwrap();
        assert_close(
            &decode_f32le(&response.values_f32le),
            &[4000.0, 4410.0, 4840.0, 5290.0],
            1.0e-6,
        );
    }

    #[test]
    fn avo_intercept_gradient_attribute_supports_chi_and_fluid_factor() {
        let chi_projection =
            avo_intercept_gradient_attribute(AvoInterceptGradientAttributeRequest {
                schema_version: 2,
                method: AvoInterceptGradientAttributeMethod::ChiProjection,
                sample_shape: vec![3],
                intercept: vec![0.10, 0.15, 0.20],
                gradient: vec![-0.20, -0.25, -0.30],
                chi_angle_deg: Some(30.0),
                intercept_scalar: None,
            })
            .unwrap();
        assert_close(
            &decode_f32le(&chi_projection.values_f32le),
            &[-0.01339746, 0.0049038157, 0.02320508],
            1.0e-6,
        );

        let fluid_factor = avo_intercept_gradient_attribute(AvoInterceptGradientAttributeRequest {
            schema_version: 2,
            method: AvoInterceptGradientAttributeMethod::FluidFactor,
            sample_shape: vec![3],
            intercept: vec![0.10, 0.15, 0.20],
            gradient: vec![-0.20, -0.25, -0.30],
            chi_angle_deg: None,
            intercept_scalar: Some(0.7),
        })
        .unwrap();
        assert_close(
            &decode_f32le(&fluid_factor.values_f32le),
            &[-0.27, -0.355, -0.44],
            1.0e-6,
        );
    }
}
