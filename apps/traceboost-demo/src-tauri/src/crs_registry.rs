use proj_sys::{
    PJ_CATEGORY_PJ_CATEGORY_CRS, PJ_TYPE, PJ_TYPE_PJ_TYPE_GEOGRAPHIC_2D_CRS,
    PJ_TYPE_PJ_TYPE_GEOGRAPHIC_3D_CRS, PJ_TYPE_PJ_TYPE_PROJECTED_CRS, PJ_TYPE_PJ_TYPE_VERTICAL_CRS,
    proj_context_create, proj_context_destroy, proj_create_from_database, proj_create_from_name,
    proj_destroy, proj_get_id_auth_name, proj_get_id_code, proj_get_name, proj_get_non_deprecated,
    proj_get_type, proj_is_deprecated, proj_list_destroy, proj_list_get, proj_list_get_count,
};
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::ptr;

const EPSG_AUTHORITY: &str = "EPSG";
const DEFAULT_SEARCH_LIMIT: usize = 24;
const MAX_IDENTIFIER_INDEX: i32 = 8;
const COMMON_EPSG_CODES: &[&str] = &[
    "4326", "3857", "32631", "32632", "25831", "25832", "23031", "23032", "28992",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateReferenceCatalogEntry {
    pub authority: String,
    pub code: String,
    pub auth_id: String,
    pub name: String,
    pub deprecated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_name: Option<String>,
    pub coordinate_reference_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCoordinateReferencesRequest {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_deprecated: Option<bool>,
    #[serde(default)]
    pub projected_only: Option<bool>,
    #[serde(default)]
    pub include_geographic: Option<bool>,
    #[serde(default)]
    pub include_vertical: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCoordinateReferencesResponse {
    pub entries: Vec<CoordinateReferenceCatalogEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveCoordinateReferenceRequest {
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub auth_id: Option<String>,
}

struct ProjContext(*mut proj_sys::PJ_CONTEXT);

impl ProjContext {
    fn new() -> Result<Self, String> {
        let ctx = unsafe { proj_context_create() };
        if ctx.is_null() {
            return Err("failed to create PROJ context".to_string());
        }
        Ok(Self(ctx))
    }

    fn raw(&self) -> *mut proj_sys::PJ_CONTEXT {
        self.0
    }
}

impl Drop for ProjContext {
    fn drop(&mut self) {
        unsafe {
            let _ = proj_context_destroy(self.0);
        }
    }
}

fn optional_string(pointer: *const std::os::raw::c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .trim()
        .to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn required_cstring(value: &str, label: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("{label} contains an unsupported NUL byte"))
}

fn preferred_identifier(
    object: *mut proj_sys::PJ,
    preferred_authority: Option<&str>,
    preferred_code: Option<&str>,
) -> Option<(String, String)> {
    let normalized_preferred_authority = preferred_authority
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());
    let normalized_preferred_code = preferred_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut first_identifier: Option<(String, String)> = None;
    let mut matching_authority_identifier: Option<(String, String)> = None;

    for index in 0..MAX_IDENTIFIER_INDEX {
        let authority = optional_string(unsafe { proj_get_id_auth_name(object, index) });
        let code = optional_string(unsafe { proj_get_id_code(object, index) });
        match (authority, code) {
            (Some(authority), Some(code)) => {
                let normalized_authority = authority.to_ascii_uppercase();
                if first_identifier.is_none() {
                    first_identifier = Some((normalized_authority.clone(), code.clone()));
                }
                if normalized_preferred_authority
                    .as_deref()
                    .is_some_and(|preferred| preferred == normalized_authority)
                {
                    if matching_authority_identifier.is_none() {
                        matching_authority_identifier =
                            Some((normalized_authority.clone(), code.clone()));
                    }
                    if normalized_preferred_code
                        .as_deref()
                        .is_some_and(|preferred| preferred.eq_ignore_ascii_case(&code))
                    {
                        return Some((normalized_authority, code));
                    }
                }
            }
            (None, None) => break,
            _ => {
                continue;
            }
        }
    }

    matching_authority_identifier.or(first_identifier)
}

fn coordinate_reference_type_label(type_: PJ_TYPE) -> &'static str {
    match type_ {
        PJ_TYPE_PJ_TYPE_PROJECTED_CRS => "projected",
        PJ_TYPE_PJ_TYPE_GEOGRAPHIC_2D_CRS => "geographic_2d",
        PJ_TYPE_PJ_TYPE_GEOGRAPHIC_3D_CRS => "geographic_3d",
        PJ_TYPE_PJ_TYPE_VERTICAL_CRS => "vertical",
        _ => "other",
    }
}

fn default_type_filter(request: &SearchCoordinateReferencesRequest) -> Vec<PJ_TYPE> {
    let mut types = Vec::new();
    if request.projected_only.unwrap_or(false) {
        return vec![PJ_TYPE_PJ_TYPE_PROJECTED_CRS];
    }

    types.push(PJ_TYPE_PJ_TYPE_PROJECTED_CRS);
    if request.include_geographic.unwrap_or(true) {
        types.push(PJ_TYPE_PJ_TYPE_GEOGRAPHIC_2D_CRS);
        types.push(PJ_TYPE_PJ_TYPE_GEOGRAPHIC_3D_CRS);
    }
    if request.include_vertical.unwrap_or(false) {
        types.push(PJ_TYPE_PJ_TYPE_VERTICAL_CRS);
    }
    types
}

fn parse_auth_id_parts(
    authority: Option<&str>,
    code: Option<&str>,
    auth_id: Option<&str>,
) -> Result<(String, String), String> {
    let parsed_from_auth_id = auth_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut parts = trimmed.splitn(2, ':');
        let authority = parts.next()?.trim();
        let code = parts.next()?.trim();
        if authority.is_empty() || code.is_empty() {
            return None;
        }
        Some((authority.to_string(), code.to_string()))
    });

    let (authority, code) = parsed_from_auth_id.unwrap_or_else(|| {
        (
            authority.unwrap_or(EPSG_AUTHORITY).trim().to_string(),
            code.unwrap_or_default().trim().to_string(),
        )
    });
    if authority.is_empty() || code.is_empty() {
        return Err("A CRS authority and code are required.".to_string());
    }
    Ok((authority.to_ascii_uppercase(), code))
}

fn resolve_authority_code_entry_internal(
    ctx: &ProjContext,
    authority: &str,
    code: &str,
) -> Result<CoordinateReferenceCatalogEntry, String> {
    let authority_upper = authority.trim().to_ascii_uppercase();
    if authority_upper != EPSG_AUTHORITY {
        return Err(format!(
            "Phase 1 CRS selection currently supports EPSG authority identifiers; received {authority_upper}:{code}."
        ));
    }

    let authority_c = required_cstring(&authority_upper, "authority")?;
    let code_c = required_cstring(code.trim(), "code")?;
    let object = unsafe {
        proj_create_from_database(
            ctx.raw(),
            authority_c.as_ptr(),
            code_c.as_ptr(),
            PJ_CATEGORY_PJ_CATEGORY_CRS,
            0,
            ptr::null(),
        )
    };
    if object.is_null() {
        return Err(format!(
            "CRS {authority_upper}:{code} was not found in the local PROJ database."
        ));
    }

    let mut object_to_use = object;
    let non_deprecated = unsafe { proj_get_non_deprecated(ctx.raw(), object) };
    if !non_deprecated.is_null() {
        let count = unsafe { proj_list_get_count(non_deprecated) };
        if count > 0 {
            let replacement = unsafe { proj_list_get(ctx.raw(), non_deprecated, 0) };
            if !replacement.is_null() {
                unsafe {
                    proj_destroy(object_to_use);
                }
                object_to_use = replacement;
            }
        }
        unsafe {
            proj_list_destroy(non_deprecated);
        }
    }

    let name = optional_string(unsafe { proj_get_name(object_to_use) })
        .ok_or_else(|| format!("CRS {authority_upper}:{code} did not expose a valid name."))?;
    let (resolved_authority, resolved_code) =
        preferred_identifier(object_to_use, Some(&authority_upper), Some(code.trim()))
            .unwrap_or_else(|| (authority_upper.clone(), code.trim().to_string()));
    let entry = CoordinateReferenceCatalogEntry {
        auth_id: format!(
            "{}:{}",
            resolved_authority.to_ascii_uppercase(),
            resolved_code
        ),
        authority: resolved_authority.to_ascii_uppercase(),
        code: resolved_code,
        name,
        deprecated: unsafe { proj_is_deprecated(object_to_use) != 0 },
        area_name: None,
        coordinate_reference_type: coordinate_reference_type_label(unsafe {
            proj_get_type(object_to_use)
        })
        .to_string(),
    };

    unsafe {
        proj_destroy(object_to_use);
    }
    Ok(entry)
}

fn search_with_name_lookup(
    ctx: &ProjContext,
    query: &str,
    request: &SearchCoordinateReferencesRequest,
) -> Result<Vec<CoordinateReferenceCatalogEntry>, String> {
    let authority_c = required_cstring(EPSG_AUTHORITY, "authority")?;
    let query_c = required_cstring(query, "query")?;
    let types = default_type_filter(request);
    let list = unsafe {
        proj_create_from_name(
            ctx.raw(),
            authority_c.as_ptr(),
            query_c.as_ptr(),
            types.as_ptr(),
            types.len(),
            1,
            request.limit.unwrap_or(DEFAULT_SEARCH_LIMIT),
            ptr::null(),
        )
    };
    if list.is_null() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let count = unsafe { proj_list_get_count(list) };
    for index in 0..count {
        let object = unsafe { proj_list_get(ctx.raw(), list, index) };
        if object.is_null() {
            continue;
        }

        let name = optional_string(unsafe { proj_get_name(object) });
        let deprecated = unsafe { proj_is_deprecated(object) != 0 };
        let type_label = coordinate_reference_type_label(unsafe { proj_get_type(object) });
        let identifier = preferred_identifier(object, Some(EPSG_AUTHORITY), None);
        if let (Some((authority, code)), Some(name)) = (identifier, name) {
            if request.include_deprecated.unwrap_or(false) || !deprecated {
                entries.push(CoordinateReferenceCatalogEntry {
                    auth_id: format!("{}:{}", authority, code),
                    authority,
                    code,
                    name,
                    deprecated,
                    area_name: None,
                    coordinate_reference_type: type_label.to_string(),
                });
            }
        }

        unsafe {
            proj_destroy(object);
        }
    }

    unsafe {
        proj_list_destroy(list);
    }
    Ok(entries)
}

fn common_entries(ctx: &ProjContext) -> Vec<CoordinateReferenceCatalogEntry> {
    let mut entries = Vec::new();
    for code in COMMON_EPSG_CODES {
        if let Ok(entry) = resolve_authority_code_entry_internal(ctx, EPSG_AUTHORITY, code) {
            entries.push(entry);
        }
    }
    entries
}

pub fn search_coordinate_references(
    request: SearchCoordinateReferencesRequest,
) -> Result<SearchCoordinateReferencesResponse, String> {
    let ctx = ProjContext::new()?;
    let query = request.query.as_deref().map(str::trim).unwrap_or_default();
    let mut entries = if query.is_empty() {
        common_entries(&ctx)
    } else {
        search_with_name_lookup(&ctx, query, &request)?
    };

    let limit = request.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    if entries.len() > limit {
        entries.truncate(limit);
    }
    Ok(SearchCoordinateReferencesResponse { entries })
}

pub fn resolve_coordinate_reference(
    request: ResolveCoordinateReferenceRequest,
) -> Result<CoordinateReferenceCatalogEntry, String> {
    let (authority, code) = parse_auth_id_parts(
        request.authority.as_deref(),
        request.code.as_deref(),
        request.auth_id.as_deref(),
    )?;
    let ctx = ProjContext::new()?;
    resolve_authority_code_entry_internal(&ctx, &authority, &code)
}

#[cfg(test)]
mod tests {
    use super::{
        ResolveCoordinateReferenceRequest, SearchCoordinateReferencesRequest,
        resolve_coordinate_reference, search_coordinate_references,
    };

    #[test]
    fn resolve_coordinate_reference_preserves_requested_epsg_projected_identifier() {
        let entry = resolve_coordinate_reference(ResolveCoordinateReferenceRequest {
            authority: None,
            code: None,
            auth_id: Some("EPSG:32632".to_string()),
        })
        .expect("expected CRS resolution to succeed");

        assert_eq!(entry.auth_id, "EPSG:32632");
        assert_eq!(entry.authority, "EPSG");
        assert_eq!(entry.code, "32632");
    }

    #[test]
    fn search_coordinate_references_default_entries_include_projected_epsg_identifier() {
        let response = search_coordinate_references(SearchCoordinateReferencesRequest {
            query: Some(String::new()),
            limit: Some(10),
            include_deprecated: Some(false),
            projected_only: Some(false),
            include_geographic: Some(true),
            include_vertical: Some(false),
        })
        .expect("expected CRS search to succeed");

        assert!(
            response
                .entries
                .iter()
                .any(|entry| entry.auth_id == "EPSG:32632"),
            "expected EPSG:32632 in {:?}",
            response
                .entries
                .iter()
                .map(|entry| entry.auth_id.as_str())
                .collect::<Vec<_>>()
        );
    }
}
