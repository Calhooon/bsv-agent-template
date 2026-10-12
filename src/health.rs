//! Pure metadata: no bindings, authentication, payments or wallet access.
pub fn build_json() -> serde_json::Value {
    let mut build = bsv_worker_build_info::build_json!();
    build
        .as_object_mut()
        .expect("build object")
        .remove("engine_rev");
    build
}

#[cfg(test)]
mod tests {
    #[test]
    fn drift_check_shape_has_captured_provenance() {
        let build = super::build_json();
        let object = build.as_object().unwrap();
        assert_eq!(object.len(), 4);
        assert!(!object.contains_key("engine_rev"));
        let rev = build["lib_rev"].as_str().unwrap();
        assert!(rev.len() == 40 && rev.bytes().all(|c| c.is_ascii_hexdigit()));
        assert!(build["dirty"].is_boolean());
        let hour = build["built_at"].as_str().unwrap();
        assert_eq!(hour.len(), 20);
        assert!(hour.ends_with(":00:00Z"));
        for name in ["bsv-rs", "bsv-middleware-cloudflare", "bsv-middleware-core"] {
            let version = build["crates"][name].as_str().unwrap();
            let lock = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock"));
            let expected = format!("name = \"{name}\"\nversion = \"{version}\"");
            assert_eq!(lock.matches(&expected).count(), 1, "{name} lock mismatch");
        }
    }
}
