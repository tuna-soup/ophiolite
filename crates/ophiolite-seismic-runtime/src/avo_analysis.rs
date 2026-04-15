use std::ops::{Add, Div, Mul, Sub};

use ophiolite_seismic::{AvoReflectivityMethod, AvoReflectivityRequest, AvoReflectivityResponse};
use rayon::prelude::*;

use crate::error::SeismicStoreError;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex64 {
    re: f64,
    im: f64,
}

impl Complex64 {
    fn sqrt(self) -> Self {
        let magnitude = (self.re * self.re + self.im * self.im).sqrt();
        let re = ((magnitude + self.re) * 0.5).sqrt();
        let im = ((magnitude - self.re) * 0.5).sqrt().copysign(self.im);
        Self { re, im }
    }
}

impl From<f64> for Complex64 {
    fn from(value: f64) -> Self {
        Self { re: value, im: 0.0 }
    }
}

impl Add for Complex64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl Sub for Complex64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl Mul for Complex64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            re: (self.re * rhs.re) - (self.im * rhs.im),
            im: (self.re * rhs.im) + (self.im * rhs.re),
        }
    }
}

impl Div for Complex64 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denominator = (rhs.re * rhs.re) + (rhs.im * rhs.im);
        Self {
            re: ((self.re * rhs.re) + (self.im * rhs.im)) / denominator,
            im: ((self.im * rhs.re) - (self.re * rhs.im)) / denominator,
        }
    }
}

pub fn avo_reflectivity(
    request: AvoReflectivityRequest,
) -> Result<AvoReflectivityResponse, SeismicStoreError> {
    let sample_count = validate_request(&request)?;
    let (intercept, gradient) = if requires_intercept_gradient(request.method) {
        let (intercept, gradient) = shuey_intercept_gradient(&request, sample_count)?;
        (Some(intercept), Some(gradient))
    } else {
        (None, None)
    };

    let angle_results = request
        .angles_deg
        .par_iter()
        .map(|angle_deg| match request.method {
            AvoReflectivityMethod::ShueyTwoTerm => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = shuey_two_term_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::ShueyThreeTerm => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = shuey_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::AkiRichards => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = aki_richards_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::AkiRichardsAlt => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = aki_richards_alt_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::Fatti => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = fatti_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::Bortfeld => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = bortfeld_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::Hilterman => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = hilterman_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::ApproxZoeppritzPp => {
                let mut real = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    real[sample_index] = approx_zoeppritz_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )? as f32;
                }
                Ok((real, None))
            }
            AvoReflectivityMethod::ZoeppritzPp => {
                let mut real = vec![0.0_f32; sample_count];
                let mut imag = vec![0.0_f32; sample_count];
                for sample_index in 0..sample_count {
                    let pp = zoeppritz_pp(
                        sample_value(&request.upper_vp_m_per_s, sample_index),
                        sample_value(&request.upper_vs_m_per_s, sample_index),
                        sample_value(&request.upper_density_g_cc, sample_index),
                        sample_value(&request.lower_vp_m_per_s, sample_index),
                        sample_value(&request.lower_vs_m_per_s, sample_index),
                        sample_value(&request.lower_density_g_cc, sample_index),
                        *angle_deg as f64,
                    )?;
                    real[sample_index] = pp.re as f32;
                    imag[sample_index] = pp.im as f32;
                }
                Ok((real, Some(imag)))
            }
        })
        .collect::<Result<Vec<_>, SeismicStoreError>>()?;

    let mut pp_real = Vec::with_capacity(request.angles_deg.len() * sample_count);
    let mut pp_imag = requires_complex_output(request.method)
        .then_some(Vec::with_capacity(request.angles_deg.len() * sample_count));
    for (real, imag) in angle_results {
        pp_real.extend(real);
        if let (Some(buffer), Some(imag)) = (pp_imag.as_mut(), imag) {
            buffer.extend(imag);
        }
    }

    Ok(AvoReflectivityResponse {
        schema_version: request.schema_version,
        method: request.method,
        sample_shape: request.sample_shape,
        angles_deg: request.angles_deg,
        pp_real_f32le: f32_vec_to_le_bytes(&pp_real),
        pp_imag_f32le: pp_imag.as_deref().map(f32_vec_to_le_bytes),
        intercept_f32le: intercept.as_deref().map(f32_vec_to_le_bytes),
        gradient_f32le: gradient.as_deref().map(f32_vec_to_le_bytes),
    })
}

fn requires_intercept_gradient(method: AvoReflectivityMethod) -> bool {
    matches!(
        method,
        AvoReflectivityMethod::ShueyTwoTerm | AvoReflectivityMethod::ShueyThreeTerm
    )
}

fn requires_complex_output(method: AvoReflectivityMethod) -> bool {
    method == AvoReflectivityMethod::ZoeppritzPp
}

fn validate_request(request: &AvoReflectivityRequest) -> Result<usize, SeismicStoreError> {
    if request.sample_shape.is_empty() {
        return Err(SeismicStoreError::Message(
            "avo reflectivity requires a non-empty sample_shape".to_string(),
        ));
    }
    let mut sample_count = 1usize;
    for axis_len in &request.sample_shape {
        if *axis_len == 0 {
            return Err(SeismicStoreError::Message(
                "avo reflectivity sample_shape dimensions must be > 0".to_string(),
            ));
        }
        sample_count = sample_count.checked_mul(*axis_len).ok_or_else(|| {
            SeismicStoreError::Message("avo reflectivity sample_shape overflows usize".to_string())
        })?;
    }
    if request.angles_deg.is_empty() {
        return Err(SeismicStoreError::Message(
            "avo reflectivity requires at least one angle".to_string(),
        ));
    }

    for (label, values) in [
        ("upper_vp_m_per_s", &request.upper_vp_m_per_s),
        ("upper_vs_m_per_s", &request.upper_vs_m_per_s),
        ("upper_density_g_cc", &request.upper_density_g_cc),
        ("lower_vp_m_per_s", &request.lower_vp_m_per_s),
        ("lower_vs_m_per_s", &request.lower_vs_m_per_s),
        ("lower_density_g_cc", &request.lower_density_g_cc),
    ] {
        if values.len() != sample_count {
            return Err(SeismicStoreError::Message(format!(
                "avo reflectivity field '{label}' length mismatch: expected {sample_count}, found {}",
                values.len()
            )));
        }
    }

    for angle_deg in &request.angles_deg {
        if !angle_deg.is_finite() || *angle_deg < 0.0 || *angle_deg >= 90.0 {
            return Err(SeismicStoreError::Message(format!(
                "avo reflectivity angles must be finite and in [0, 90), found {angle_deg}"
            )));
        }
    }

    for (label, values) in [
        ("upper_vp_m_per_s", &request.upper_vp_m_per_s),
        ("upper_vs_m_per_s", &request.upper_vs_m_per_s),
        ("upper_density_g_cc", &request.upper_density_g_cc),
        ("lower_vp_m_per_s", &request.lower_vp_m_per_s),
        ("lower_vs_m_per_s", &request.lower_vs_m_per_s),
        ("lower_density_g_cc", &request.lower_density_g_cc),
    ] {
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() || *value <= 0.0 {
                return Err(SeismicStoreError::Message(format!(
                    "avo reflectivity field '{label}' contains invalid value {value} at index {index}"
                )));
            }
        }
    }

    Ok(sample_count)
}

fn shuey_intercept_gradient(
    request: &AvoReflectivityRequest,
    sample_count: usize,
) -> Result<(Vec<f32>, Vec<f32>), SeismicStoreError> {
    let mut intercept = vec![0.0_f32; sample_count];
    let mut gradient = vec![0.0_f32; sample_count];
    for sample_index in 0..sample_count {
        let (r0, g, _) = shuey_terms(
            sample_value(&request.upper_vp_m_per_s, sample_index),
            sample_value(&request.upper_vs_m_per_s, sample_index),
            sample_value(&request.upper_density_g_cc, sample_index),
            sample_value(&request.lower_vp_m_per_s, sample_index),
            sample_value(&request.lower_vs_m_per_s, sample_index),
            sample_value(&request.lower_density_g_cc, sample_index),
        )?;
        intercept[sample_index] = r0 as f32;
        gradient[sample_index] = g as f32;
    }
    Ok((intercept, gradient))
}

fn sample_value(values: &[f32], index: usize) -> f64 {
    values[index] as f64
}

fn shuey_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let theta = angle_deg.to_radians();
    let (r0, g, f) = shuey_terms(vp1, vs1, rho1, vp2, vs2, rho2)?;
    let sin2 = theta.sin().powi(2);
    let tan2 = theta.tan().powi(2);
    Ok(r0 + (g * sin2) + (f * (tan2 - sin2)))
}

fn shuey_two_term_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let theta = angle_deg.to_radians();
    let (r0, g, _) = shuey_terms(vp1, vs1, rho1, vp2, vs2, rho2)?;
    Ok(r0 + (g * theta.sin().powi(2)))
}

fn shuey_terms(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
) -> Result<(f64, f64, f64), SeismicStoreError> {
    let vp = 0.5 * (vp1 + vp2);
    let vs = 0.5 * (vs1 + vs2);
    let rho = 0.5 * (rho1 + rho2);
    if vp <= 0.0 || vs <= 0.0 || rho <= 0.0 {
        return Err(SeismicStoreError::Message(
            "shuey reflectivity requires positive average elastic properties".to_string(),
        ));
    }

    let dvp = vp2 - vp1;
    let dvs = vs2 - vs1;
    let drho = rho2 - rho1;
    let r0 = 0.5 * ((dvp / vp) + (drho / rho));
    let gradient =
        (0.5 * (dvp / vp)) - (2.0 * (vs * vs / (vp * vp)) * ((drho / rho) + (2.0 * dvs / vs)));
    let curvature = 0.5 * (dvp / vp);
    Ok((r0, gradient, curvature))
}

fn aki_richards_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let vp = 0.5 * (vp1 + vp2);
    let vs = 0.5 * (vs1 + vs2);
    let rho = 0.5 * (rho1 + rho2);
    if vp <= 0.0 || vs <= 0.0 || rho <= 0.0 || vp1 <= 0.0 {
        return Err(SeismicStoreError::Message(
            "aki-richards reflectivity requires positive elastic properties".to_string(),
        ));
    }

    let theta1 = angle_deg.to_radians();
    let theta2 = asin_checked(
        (vp2 / vp1) * theta1.sin(),
        "aki-richards reflectivity encountered a post-critical angle",
    )?;
    let meantheta = 0.5 * (theta1 + theta2);
    let dvp = vp2 - vp1;
    let dvs = vs2 - vs1;
    let drho = rho2 - rho1;
    let mean_cos_sq = meantheta.cos().powi(2);
    if mean_cos_sq <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "aki-richards reflectivity encountered an invalid incidence angle".to_string(),
        ));
    }

    let sin_theta_sq = theta1.sin().powi(2);
    let w = 0.5 * (drho / rho);
    let x = 2.0 * (vs / vp1).powi(2) * (drho / rho);
    let y = 0.5 * (dvp / vp);
    let z = 4.0 * (vs / vp1).powi(2) * (dvs / vs);
    Ok(w - (x * sin_theta_sq) + (y / mean_cos_sq) - (z * sin_theta_sq))
}

fn aki_richards_alt_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let vp = 0.5 * (vp1 + vp2);
    let vs = 0.5 * (vs1 + vs2);
    let rho = 0.5 * (rho1 + rho2);
    if vp <= 0.0 || vs <= 0.0 || rho <= 0.0 || vp1 <= 0.0 {
        return Err(SeismicStoreError::Message(
            "aki-richards-alt reflectivity requires positive elastic properties".to_string(),
        ));
    }

    let theta1 = angle_deg.to_radians();
    let theta2 = asin_checked(
        (vp2 / vp1) * theta1.sin(),
        "aki-richards-alt reflectivity encountered a post-critical angle",
    )?;
    let theta = 0.5 * (theta1 + theta2);
    let dvp = vp2 - vp1;
    let dvs = vs2 - vs1;
    let drho = rho2 - rho1;
    let sin2 = theta.sin().powi(2);
    let tan2 = theta.tan().powi(2);
    Ok(0.5 * ((dvp / vp) + (drho / rho))
        + ((0.5 * (dvp / vp)) - (2.0 * (vs / vp).powi(2) * ((drho / rho) + (2.0 * dvs / vs))))
            * sin2
        + (0.5 * (dvp / vp) * (tan2 - sin2)))
}

fn fatti_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let vp = 0.5 * (vp1 + vp2);
    let vs = 0.5 * (vs1 + vs2);
    let rho = 0.5 * (rho1 + rho2);
    if vp <= 0.0 || vs <= 0.0 || rho <= 0.0 {
        return Err(SeismicStoreError::Message(
            "fatti reflectivity requires positive elastic properties".to_string(),
        ));
    }

    let theta = angle_deg.to_radians();
    let drho = rho2 - rho1;
    let dip = reflection_contrast(vp1 * rho1, vp2 * rho2)?;
    let dis = reflection_contrast(vs1 * rho1, vs2 * rho2)?;
    let density_contrast = drho / rho;
    let sin2 = theta.sin().powi(2);
    let tan2 = theta.tan().powi(2);
    let vpvs2 = (vs / vp).powi(2);
    Ok(((1.0 + tan2) * dip)
        - (8.0 * vpvs2 * dis * sin2)
        - ((0.5 * tan2) - (2.0 * vpvs2 * sin2)) * density_contrast)
}

fn bortfeld_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let vp = 0.5 * (vp1 + vp2);
    let vs = 0.5 * (vs1 + vs2);
    let rho = 0.5 * (rho1 + rho2);
    if vp <= 0.0 || vs <= 0.0 || rho <= 0.0 {
        return Err(SeismicStoreError::Message(
            "bortfeld reflectivity requires positive elastic properties".to_string(),
        ));
    }

    let theta = angle_deg.to_radians();
    let dvp = vp2 - vp1;
    let dvs = vs2 - vs1;
    let drho = rho2 - rho1;
    let k = (2.0 * vs / vp).powi(2);
    let rsh = 0.5 * ((dvp / vp) - (k * drho / rho) - (2.0 * k * dvs / vs));
    let sin2 = theta.sin().powi(2);
    Ok((0.5 * ((dvp / vp) + (drho / rho)))
        + (rsh * sin2)
        + (0.5 * (dvp / vp) * theta.tan().powi(2) * sin2))
}

fn hilterman_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let theta = angle_deg.to_radians();
    let rp0 = reflection_contrast(vp1 * rho1, vp2 * rho2)?;
    let pr1 = poissons_ratio(vp1, vs1)?;
    let pr2 = poissons_ratio(vp2, vs2)?;
    let pravg = 0.5 * (pr1 + pr2);
    let denominator = (1.0 - pravg).powi(2);
    if denominator <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "hilterman reflectivity encountered an invalid Poisson ratio".to_string(),
        ));
    }
    let pr = (pr2 - pr1) / denominator;
    Ok((rp0 * theta.cos().powi(2)) + (pr * theta.sin().powi(2)))
}

fn approx_zoeppritz_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<f64, SeismicStoreError> {
    let theta1 = angle_deg.to_radians();
    let p = theta1.sin() / vp1;
    let theta2 = asin_checked(
        p * vp2,
        "approx-zoeppritz reflectivity encountered a post-critical P-wave angle",
    )?;
    let phi1 = asin_checked(
        p * vs1,
        "approx-zoeppritz reflectivity encountered a post-critical reflected S-wave angle",
    )?;
    let phi2 = asin_checked(
        p * vs2,
        "approx-zoeppritz reflectivity encountered a post-critical transmitted S-wave angle",
    )?;

    let sin_phi1_sq = phi1.sin().powi(2);
    let sin_phi2_sq = phi2.sin().powi(2);
    let a = (rho2 * (1.0 - (2.0 * sin_phi2_sq))) - (rho1 * (1.0 - (2.0 * sin_phi1_sq)));
    let b = (rho2 * (1.0 - (2.0 * sin_phi2_sq))) + (2.0 * rho1 * sin_phi1_sq);
    let c = (rho1 * (1.0 - (2.0 * sin_phi1_sq))) + (2.0 * rho2 * sin_phi2_sq);
    let d = 2.0 * ((rho2 * vs2 * vs2) - (rho1 * vs1 * vs1));

    let e = (b * theta1.cos() / vp1) + (c * theta2.cos() / vp2);
    let f = (b * phi1.cos() / vs1) + (c * phi2.cos() / vs2);
    let g = a - (d * theta1.cos() / vp1 * phi2.cos() / vs2);
    let h = a - (d * theta2.cos() / vp2 * phi1.cos() / vs1);
    let denominator = (e * f) + (g * h * p * p);
    if denominator.abs() <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "approx-zoeppritz reflectivity denominator collapsed to zero".to_string(),
        ));
    }

    Ok(((f * ((b * theta1.cos() / vp1) - (c * theta2.cos() / vp2)))
        - (h * p * p * (a + (d * theta1.cos() / vp1 * phi2.cos() / vs2))))
        / denominator)
}

fn reflection_contrast(upper: f64, lower: f64) -> Result<f64, SeismicStoreError> {
    let denominator = lower + upper;
    if denominator.abs() <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "reflectivity contrast denominator collapsed to zero".to_string(),
        ));
    }
    Ok((lower - upper) / denominator)
}

fn poissons_ratio(vp: f64, vs: f64) -> Result<f64, SeismicStoreError> {
    let vp_sq = vp * vp;
    let vs_sq = vs * vs;
    let denominator = 2.0 * (vp_sq - vs_sq);
    if denominator.abs() <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "poisson ratio requires vp and vs to define a valid isotropic medium".to_string(),
        ));
    }
    Ok((vp_sq - (2.0 * vs_sq)) / denominator)
}

fn asin_checked(value: f64, error_message: &str) -> Result<f64, SeismicStoreError> {
    if !value.is_finite() || value.abs() > 1.0 + 1.0e-12 {
        return Err(SeismicStoreError::Message(error_message.to_string()));
    }
    Ok(value.clamp(-1.0, 1.0).asin())
}

fn zoeppritz_pp(
    vp1: f64,
    vs1: f64,
    rho1: f64,
    vp2: f64,
    vs2: f64,
    rho2: f64,
    angle_deg: f64,
) -> Result<Complex64, SeismicStoreError> {
    let theta = angle_deg.to_radians();
    let ray_parameter = theta.sin() / vp1;
    let p2 = ray_parameter * ray_parameter;
    let svel1 = if vs1 <= 1.0e-6 { 0.1 } else { vs1 };
    let svel2 = if vs2 <= 1.0e-6 { 0.1 } else { vs2 };
    if !svel1.is_finite() || !svel2.is_finite() {
        return Err(SeismicStoreError::Message(
            "zoeppritz reflectivity encountered invalid shear velocity".to_string(),
        ));
    }

    let l1s2 = svel1 * svel1;
    let l2s2 = svel2 * svel2;
    let l1p2 = vp1 * vp1;
    let l2p2 = vp2 * vp2;

    let a =
        Complex64::from((rho2 * (1.0 - (2.0 * l2s2 * p2))) - (rho1 * (1.0 - (2.0 * l1s2 * p2))));
    let b = Complex64::from((rho2 * (1.0 - (2.0 * l2s2 * p2))) + (rho1 * 2.0 * l1s2 * p2));
    let c = Complex64::from((rho1 * (1.0 - (2.0 * l1s2 * p2))) + (rho2 * 2.0 * l2s2 * p2));
    let d = Complex64::from(2.0 * ((rho2 * l2s2) - (rho1 * l1s2)));

    let pzi1 = Complex64::from((1.0 / l1p2) - p2).sqrt();
    let pzi2 = Complex64::from((1.0 / l2p2) - p2).sqrt();
    let pzj1 = Complex64::from((1.0 / l1s2) - p2).sqrt();
    let pzj2 = Complex64::from((1.0 / l2s2) - p2).sqrt();

    let ee = (b * pzi1) + (c * pzi2);
    let ff = (b * pzj1) + (c * pzj2);
    let gg = a - ((d * pzi1) * pzj2);
    let hh = a - ((d * pzi2) * pzj1);
    let dd = (ee * ff) + ((gg * hh) * Complex64::from(p2));
    if dd.re.abs() <= f64::EPSILON && dd.im.abs() <= f64::EPSILON {
        return Err(SeismicStoreError::Message(
            "zoeppritz reflectivity denominator collapsed to zero".to_string(),
        ));
    }

    Ok(
        (((b * pzi1) - (c * pzi2)) * ff - ((a + ((d * pzi1) * pzj2)) * hh * Complex64::from(p2)))
            / dd,
    )
}

fn f32_vec_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
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
            .chunks_exact(std::mem::size_of::<f32>())
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("4 bytes")))
            .collect()
    }

    fn reference_request(method: AvoReflectivityMethod) -> AvoReflectivityRequest {
        AvoReflectivityRequest {
            schema_version: 2,
            method,
            sample_shape: vec![1],
            angles_deg: vec![0.0, 10.0, 20.0, 30.0, 40.0],
            upper_vp_m_per_s: vec![2190.0],
            upper_vs_m_per_s: vec![716.0],
            upper_density_g_cc: vec![2.118],
            lower_vp_m_per_s: vec![2760.0],
            lower_vs_m_per_s: vec![1473.0],
            lower_density_g_cc: vec![2.229],
        }
    }

    fn assert_close(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= tolerance,
                "row {index}: expected {expected}, found {actual}"
            );
        }
    }

    #[test]
    fn shuey_three_term_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::ShueyThreeTerm))
            .expect("shuey response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[
                0.14068636,
                0.12735021,
                0.09031595,
                0.038819134,
                -0.010030269,
            ],
            1.0e-6,
        );
        assert_close(
            &decode_f32le(
                response
                    .intercept_f32le
                    .as_deref()
                    .expect("intercept bytes"),
            ),
            &[0.14068636],
            1.0e-6,
        );
        assert_close(
            &decode_f32le(response.gradient_f32le.as_deref().expect("gradient bytes")),
            &[-0.44585276],
            1.0e-6,
        );
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn shuey_two_term_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::ShueyTwoTerm))
            .expect("shuey two-term response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14068636, 0.12724227, 0.0885315, 0.029223174, -0.043529257],
            1.0e-6,
        );
        assert_close(
            &decode_f32le(
                response
                    .intercept_f32le
                    .as_deref()
                    .expect("intercept bytes"),
            ),
            &[0.14068636],
            1.0e-6,
        );
        assert_close(
            &decode_f32le(response.gradient_f32le.as_deref().expect("gradient bytes")),
            &[-0.44585276],
            1.0e-6,
        );
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn aki_richards_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::AkiRichards))
            .expect("aki-richards response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14068636, 0.12369561, 0.07715111, 0.016071362, -0.02245564],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn aki_richards_alt_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::AkiRichardsAlt))
            .expect("aki-richards-alt response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[
                0.14068636,
                0.12368413,
                0.07695536,
                0.014946876,
                -0.026986497,
            ],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn fatti_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::Fatti))
            .expect("fatti response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14027391, 0.12707828, 0.09044373, 0.03954054, -0.008631967],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn bortfeld_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::Bortfeld))
            .expect("bortfeld response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[
                0.14068636,
                0.12735021,
                0.09031595,
                0.038819134,
                -0.010030269,
            ],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn hilterman_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::Hilterman))
            .expect("hilterman response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14027391, 0.12544435, 0.08274433, 0.017324114, -0.06292567],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn approx_zoeppritz_matches_pylops_reference_values() {
        let response =
            avo_reflectivity(reference_request(AvoReflectivityMethod::ApproxZoeppritzPp))
                .expect("approx zoeppritz response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14027391, 0.12944523, 0.09899729, 0.056503464, 0.02468089],
            1.0e-6,
        );
        assert!(response.intercept_f32le.is_none());
        assert!(response.gradient_f32le.is_none());
        assert!(response.pp_imag_f32le.is_none());
    }

    #[test]
    fn zoeppritz_matches_bruges_reference_values() {
        let response = avo_reflectivity(reference_request(AvoReflectivityMethod::ZoeppritzPp))
            .expect("zoeppritz response");

        assert_close(
            &decode_f32le(&response.pp_real_f32le),
            &[0.14027391, 0.12944523, 0.09899729, 0.056503464, 0.02468089],
            1.0e-6,
        );
        assert_close(
            &decode_f32le(response.pp_imag_f32le.as_deref().expect("imaginary bytes")),
            &[0.0, 0.0, 0.0, 0.0, 0.0],
            1.0e-6,
        );
    }

    #[test]
    fn avo_reflectivity_preserves_angle_major_packing_for_multidimensional_samples() {
        let request = AvoReflectivityRequest {
            schema_version: 2,
            method: AvoReflectivityMethod::ShueyTwoTerm,
            sample_shape: vec![2, 2],
            angles_deg: vec![0.0, 20.0],
            upper_vp_m_per_s: vec![2190.0, 2200.0, 2210.0, 2220.0],
            upper_vs_m_per_s: vec![716.0, 720.0, 724.0, 728.0],
            upper_density_g_cc: vec![2.118, 2.12, 2.122, 2.124],
            lower_vp_m_per_s: vec![2760.0, 2770.0, 2780.0, 2790.0],
            lower_vs_m_per_s: vec![1473.0, 1478.0, 1483.0, 1488.0],
            lower_density_g_cc: vec![2.229, 2.231, 2.233, 2.235],
        };

        let response = avo_reflectivity(request.clone()).expect("multidimensional response");
        let actual = decode_f32le(&response.pp_real_f32le);
        let mut expected = Vec::new();
        for angle_deg in &request.angles_deg {
            for sample_index in 0..request.upper_vp_m_per_s.len() {
                expected.push(
                    shuey_two_term_pp(
                        request.upper_vp_m_per_s[sample_index] as f64,
                        request.upper_vs_m_per_s[sample_index] as f64,
                        request.upper_density_g_cc[sample_index] as f64,
                        request.lower_vp_m_per_s[sample_index] as f64,
                        request.lower_vs_m_per_s[sample_index] as f64,
                        request.lower_density_g_cc[sample_index] as f64,
                        *angle_deg as f64,
                    )
                    .expect("sample reflectivity") as f32,
                );
            }
        }

        assert_close(&actual, &expected, 1.0e-6);
    }

    #[test]
    fn avo_reflectivity_rejects_mismatched_lengths() {
        let mut request = reference_request(AvoReflectivityMethod::ShueyThreeTerm);
        request.lower_density_g_cc.clear();
        let error = avo_reflectivity(request).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("lower_density_g_cc' length mismatch")
        );
    }
}
