//! Authored punctual lights. Linear RGB and physical intensity units follow
//! glTF KHR_lights_punctual; import of that extension is a separate capability.
use incant_types::{DirectionalLight, PointLight, SpotLight};

pub(crate) fn annotate_schemas(
    registry: &mut std::collections::BTreeMap<String, serde_json::Value>,
) {
    use serde_json::json;
    registry
        .get_mut("DirectionalLight")
        .expect("registered light schema")["$defs"]["DirectionalShadows"]["properties"]["distance"]
        ["x-incant-unit"] = json!("m");
    for kind in ["DirectionalLight", "PointLight", "SpotLight"] {
        let properties =
            &mut registry.get_mut(kind).expect("registered light schema")["properties"];
        properties["color"]["x-incant-widget"] = json!("rgb");
        properties["color"]["items"]["minimum"] = json!(0);
        properties["color"]["items"]["maximum"] = json!(1);
        properties["intensity"]["x-incant-unit"] = json!(if kind == "DirectionalLight" {
            "lx"
        } else {
            "cd"
        });
        if kind != "DirectionalLight" {
            properties["range"]["x-incant-unit"] = json!("m");
        }
        if kind == "SpotLight" {
            for angle in ["inner_degrees", "outer_degrees"] {
                properties[angle]["x-incant-unit"] = json!("°");
            }
        }
    }
}

pub(crate) fn validate(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    let (color, intensity, range) = match kind {
        "DirectionalLight" => {
            let light: DirectionalLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if light
                .shadows
                .as_ref()
                .is_some_and(|s| !(0.01..=10000.0).contains(&s.distance))
            {
                return Err("directional shadow distance must be 0.01..10000 meters".into());
            }
            (light.color, light.intensity, None)
        }
        "PointLight" => {
            let light: PointLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            (light.color, light.intensity, Some(light.range))
        }
        "SpotLight" => {
            let light: SpotLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !(0.0..=89.9).contains(&light.inner_degrees)
                || !(0.1..=89.9).contains(&light.outer_degrees)
                || light.inner_degrees >= light.outer_degrees
            {
                return Err("spot angles require 0 <= inner < outer <= 89.9 degrees".into());
            }
            (light.color, light.intensity, Some(light.range))
        }
        _ => return Err("unregistered light type".into()),
    };
    if !color.iter().all(|v| (0.0..=1.0).contains(v)) || !(0.0..=1_000_000.0).contains(&intensity) {
        return Err("light requires linear RGB in 0..1 and intensity in 0..1000000".into());
    }
    if range.is_some_and(|range| !(0.001..=10000.0).contains(&range)) {
        return Err("light range must be 0.001..10000 meters".into());
    }
    Ok(())
}
