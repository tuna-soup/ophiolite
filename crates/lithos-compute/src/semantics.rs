use lithos_core::CanonicalAlias;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CurveSemanticType {
    GammaRay,
    BulkDensity,
    NeutronPorosity,
    DeepResistivity,
    MediumResistivity,
    ShallowResistivity,
    Sonic,
    ShearSonic,
    Depth,
    Time,
    PVelocity,
    SVelocity,
    AcousticImpedance,
    PoissonsRatio,
    VShale,
    Computed,
    Unknown,
}

impl CurveSemanticType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GammaRay => "Gamma Ray",
            Self::BulkDensity => "Bulk Density",
            Self::NeutronPorosity => "Neutron Porosity",
            Self::DeepResistivity => "Deep Resistivity",
            Self::MediumResistivity => "Medium Resistivity",
            Self::ShallowResistivity => "Shallow Resistivity",
            Self::Sonic => "Sonic",
            Self::ShearSonic => "Shear Sonic",
            Self::Depth => "Depth",
            Self::Time => "Time",
            Self::PVelocity => "P-wave Velocity",
            Self::SVelocity => "S-wave Velocity",
            Self::AcousticImpedance => "Acoustic Impedance",
            Self::PoissonsRatio => "Poisson's Ratio",
            Self::VShale => "VShale",
            Self::Computed => "Computed",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetSemanticFamily {
    Log,
    Trajectory,
    TopSet,
    PressureObservation,
    DrillingObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurveSemanticSource {
    Derived,
    Override,
    Computed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurveSemanticDescriptor {
    pub curve_name: String,
    pub original_mnemonic: String,
    pub unit: Option<String>,
    pub semantic_type: CurveSemanticType,
    pub source: CurveSemanticSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurveBindingCandidate {
    pub curve_name: String,
    pub original_mnemonic: String,
    pub semantic_type: CurveSemanticType,
    pub unit: Option<String>,
}

pub fn default_curve_semantics(
    curves: impl IntoIterator<Item = CurveSemanticDescriptor>,
) -> Vec<CurveSemanticDescriptor> {
    curves.into_iter().collect()
}

pub fn classify_curve_semantic(
    alias: &CanonicalAlias,
    raw_mnemonic: &str,
    unit: Option<&str>,
    is_index: bool,
) -> CurveSemanticType {
    if is_index {
        return match alias.mnemonic.as_deref() {
            Some("time") => CurveSemanticType::Time,
            _ => CurveSemanticType::Depth,
        };
    }

    match alias.mnemonic.as_deref() {
        Some("gamma_ray") => return CurveSemanticType::GammaRay,
        Some("bulk_density") => return CurveSemanticType::BulkDensity,
        Some("neutron_porosity") => return CurveSemanticType::NeutronPorosity,
        Some("deep_resistivity") => return CurveSemanticType::DeepResistivity,
        Some("medium_resistivity") => return CurveSemanticType::MediumResistivity,
        Some("shallow_resistivity") => return CurveSemanticType::ShallowResistivity,
        Some("depth") => return CurveSemanticType::Depth,
        Some("time") => return CurveSemanticType::Time,
        _ => {}
    }

    let mnemonic = raw_mnemonic.trim().to_ascii_uppercase();
    let normalized_unit = unit.unwrap_or_default().trim().to_ascii_lowercase();

    match mnemonic.as_str() {
        "DT" | "DTC" | "AC" => CurveSemanticType::Sonic,
        "DTS" | "DTSM" => CurveSemanticType::ShearSonic,
        "VP" | "PVEL" | "P_VEL" => CurveSemanticType::PVelocity,
        "VS" | "SVEL" | "S_VEL" => CurveSemanticType::SVelocity,
        "AI" | "AIMP" => CurveSemanticType::AcousticImpedance,
        "PR" | "NU" | "POISSON" => CurveSemanticType::PoissonsRatio,
        _ if normalized_unit.contains("us/ft") || normalized_unit.contains("us/m") => {
            CurveSemanticType::Sonic
        }
        _ => CurveSemanticType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_uses_alias_and_mnemonic_heuristics() {
        let alias = CanonicalAlias {
            mnemonic: Some("gamma_ray".to_string()),
            unit_hint: Some("gapi".to_string()),
        };
        assert_eq!(
            classify_curve_semantic(&alias, "GR", Some("gAPI"), false),
            CurveSemanticType::GammaRay
        );

        let unknown = CanonicalAlias {
            mnemonic: None,
            unit_hint: None,
        };
        assert_eq!(
            classify_curve_semantic(&unknown, "DT", Some("us/ft"), false),
            CurveSemanticType::Sonic
        );
    }
}
