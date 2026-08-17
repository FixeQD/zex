//! PipeWire `Props` POD codec for volume and mute state

use pipewire::spa::param::ParamType;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Pod, Property, Value, ValueArray};
use pipewire::spa::utils::SpaTypes;

// SPA_PROP values from spa/param/props.h
const SPA_PROP_VOLUME: u32 = 0x10003;
const SPA_PROP_MUTE: u32 = 0x10004;
const SPA_PROP_CHANNEL_VOLUMES: u32 = 0x10008;

/// Serialize volume + mute into a `Props` POD usable with `set_param`
pub fn build_volume_pod(volume: f32, muted: bool) -> Vec<u8> {
    let obj = Object {
        type_: SpaTypes::ObjectParamProps.as_raw(),
        id: ParamType::Props.as_raw(),
        properties: vec![
            Property::new(SPA_PROP_VOLUME, Value::Float(volume)),
            Property::new(SPA_PROP_MUTE, Value::Bool(muted)),
            Property::new(
                SPA_PROP_CHANNEL_VOLUMES,
                Value::ValueArray(ValueArray::Float(vec![volume])),
            ),
        ],
    };
    PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(obj))
        .unwrap()
        .0
        .into_inner()
}

/// Parse volume and mute state out of a `Props` POD
pub fn parse_volume_pod(pod: &Pod) -> Option<(f32, bool)> {
    let (_, value) = PodDeserializer::deserialize_from::<Value>(pod.as_bytes()).ok()?;
    match value {
        Value::Object(obj) => {
            let mut volume = None;
            let mut muted = None;
            for prop in &obj.properties {
                match prop.key {
                    SPA_PROP_VOLUME => {
                        if let Value::Float(v) = prop.value {
                            volume = Some(v);
                        }
                    }
                    SPA_PROP_CHANNEL_VOLUMES => {
                        if volume.is_some() {
                            continue;
                        }
                        if let Value::ValueArray(ValueArray::Float(vals)) = &prop.value {
                            volume = vals.iter().copied().reduce(f32::max);
                        }
                    }
                    SPA_PROP_MUTE => {
                        if let Value::Bool(m) = prop.value {
                            muted = Some(m);
                        }
                    }
                    _ => {}
                }
            }
            match (volume, muted) {
                (Some(volume), Some(muted)) => Some((volume, muted)),
                (Some(volume), None) => Some((volume, false)),
                _ => None,
            }
        }
        _ => None,
    }
}
