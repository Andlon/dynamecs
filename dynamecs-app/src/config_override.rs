use eyre::{eyre, WrapErr};
use serde_json::{Map, Value};
use tracing::info;

struct InvalidOverride;

fn recursively_apply_config_override(
    config_part: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), InvalidOverride> {
    if let Value::Object(obj) = config_part {
        let (head, tail) = path
            .split_once(".")
            .map(|(head, tail)| (head, Some(tail)))
            .unwrap_or_else(|| (path, None));
        if let Some(val) = obj.get_mut(head) {
            if let Some(tail) = tail {
                // If we have a tail, then we have to keep digging down in the hierarchy
                recursively_apply_config_override(val, tail, value)
            } else {
                // Otherwise we arrived at the right spot, we're done!
                *val = value;
                Ok(())
            }
        } else {
            let mut new_obj_map = Map::new();
            if let Some(tail) = tail {
                new_obj_map.insert(tail.to_string(), serde_json::Value::Object(Map::new()));
                let mut new_obj = serde_json::Value::Object(new_obj_map);
                recursively_apply_config_override(&mut new_obj, tail, value)?;
                obj.insert(head.to_string(), new_obj);
                Ok(())
            } else {
                obj.insert(head.to_string(), value);
                Ok(())
            }
        }
    } else {
        Err(InvalidOverride)
    }
}

pub fn apply_config_override(config_json: &mut serde_json::Value, config_override: &str) -> eyre::Result<()> {
    let (path, value) = config_override.split_once("=").ok_or_else(|| {
        eyre!(
            "invalid config override '{}'. Overrides take the form <path>=<value>, see --help.",
            config_override
        )
    })?;

    let value_as_json: serde_json::Value = json5::from_str(value).wrap_err_with(|| {
        format!(
            "failed to deserialize override value for override \"{config_override}\". \
            The provided value \"{value}\" does not appear to be valid JSON5"
        )
    })?;
    recursively_apply_config_override(config_json, path, value_as_json)
        .or_else(|_| Err(eyre!("invalid override {config_override} for config")))?;
    Ok(())
}

pub fn apply_config_overrides(
    mut config_json: serde_json::Value,
    overrides: &[String],
) -> eyre::Result<serde_json::Value> {
    for config_override in overrides.iter() {
        info!(target: "dynamecs_app", "Applying config override: {config_override}");
        apply_config_override(&mut config_json, config_override)?;
    }

    Ok(config_json)
}

#[cfg(test)]
mod tests {
    use crate::config_override::apply_config_override;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// Just a mock struct to contain some bogus info for unit tests
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct MeshStats {
        num_verts: usize,
        map: HashMap<String, usize>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct MockConfig {
        resolution: usize,
        name: String,
        stats: MeshStats,
    }

    #[test]
    fn test_basic_override() {
        macro_rules! assert_override_eq {
            ($input_cfg:expr, override = $override:expr, => $expected_cfg:expr) => {{
                let mut config_json = serde_json::to_value($input_cfg.clone()).unwrap();
                apply_config_override(&mut config_json, $override).unwrap();
                let overridden_config: MockConfig = serde_json::from_value(config_json).unwrap();
                assert_eq!(&overridden_config, &$expected_cfg);
            }};
        }

        let input_cfg = MockConfig {
            resolution: 4,
            name: "Bear".to_string(),
            stats: MeshStats {
                num_verts: 100,
                map: vec![("boundary".to_string(), 10), ("interior".to_string(), 5)]
                    .into_iter()
                    .collect(),
            },
        };

        assert_override_eq!(input_cfg,
                            override = "resolution=3",
                            => MockConfig { resolution: 3, .. input_cfg.clone() });
        assert_override_eq!(input_cfg,
                            override = r#"name="Cat""#,
                            => MockConfig { name: "Cat".to_string(), .. input_cfg.clone() });
        assert_override_eq!(input_cfg,
                            override = r#"stats.num_verts=5"#,
                            => MockConfig { stats: MeshStats { num_verts: 5, .. input_cfg.stats.clone() }, .. input_cfg.clone() });

        {
            // More complex override, need some more work to construct the expected result
            let mut new_map = input_cfg.stats.map.clone();
            new_map.insert("boundary".to_string(), 5);
            let expected = MockConfig {
                stats: MeshStats {
                    map: new_map,
                    ..input_cfg.stats.clone()
                },
                ..input_cfg.clone()
            };
            assert_override_eq!(input_cfg,
                            override = r#"stats.map.boundary=5"#,
                            => expected);
        }

        {
            // Actually *insert* data that was not there already by inserting something into
            // a hash map
            let mut new_map = input_cfg.stats.map.clone();
            new_map.insert("middle".to_string(), 7);
            let expected = MockConfig {
                stats: MeshStats {
                    map: new_map,
                    ..input_cfg.stats.clone()
                },
                ..input_cfg.clone()
            };
            assert_override_eq!(input_cfg,
                            override = r#"stats.map.middle=7"#,
                            => expected);
        }
    }
}
