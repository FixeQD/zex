//! Unit tests for the PipeWire volume POD parser and builder.

use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Pod, Property, Value, ValueArray};
use pipewire::spa::utils::SpaTypes;
use zex_services::audio::volume::{build_volume_pod, parse_volume_pod};

const SPA_PROP_VOLUME: u32 = 0x10003;
const SPA_PROP_MUTE: u32 = 0x10004;
const SPA_PROP_CHANNEL_VOLUMES: u32 = 0x10008;

fn pod_bytes_for(volume: Option<f32>, muted: Option<bool>) -> Vec<u8> {
    let mut properties = Vec::new();
    if let Some(volume) = volume {
        properties.push(Property::new(SPA_PROP_VOLUME, Value::Float(volume)));
    }
    if let Some(muted) = muted {
        properties.push(Property::new(SPA_PROP_MUTE, Value::Bool(muted)));
    }
    let obj = Object {
        type_: SpaTypes::ObjectParamProps.as_raw(),
        id: ParamType::Props.as_raw(),
        properties,
    };
    PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(obj))
        .unwrap()
        .0
        .into_inner()
}

#[test]
fn volume_pod_roundtrip() {
    let bytes = build_volume_pod(0.5, true);
    let pod = Pod::from_bytes(&bytes).unwrap();
    assert_eq!(parse_volume_pod(pod), Some((0.5, true)));
}

#[test]
fn parse_volume_and_mute() {
    let bytes = pod_bytes_for(Some(0.3), Some(false));
    assert_eq!(
        parse_volume_pod(Pod::from_bytes(&bytes).unwrap()),
        Some((0.3, false))
    );
    let bytes = pod_bytes_for(Some(0.0), Some(true));
    assert_eq!(
        parse_volume_pod(Pod::from_bytes(&bytes).unwrap()),
        Some((0.0, true))
    );
}

#[test]
fn volume_without_mute_defaults_to_unmuted() {
    let bytes = pod_bytes_for(Some(0.7), None);
    assert_eq!(
        parse_volume_pod(Pod::from_bytes(&bytes).unwrap()),
        Some((0.7, false))
    );
}

#[test]
fn mute_without_volume_is_rejected() {
    let bytes = pod_bytes_for(None, Some(true));
    assert_eq!(parse_volume_pod(Pod::from_bytes(&bytes).unwrap()), None);
}

#[test]
fn empty_object_is_rejected() {
    let bytes = pod_bytes_for(None, None);
    assert_eq!(parse_volume_pod(Pod::from_bytes(&bytes).unwrap()), None);
}

#[test]
fn garbage_bytes_are_rejected() {
    let bytes = vec![0xff, 0x00, 0x42];
    // Not a valid pod header, so construction itself fails.
    assert!(Pod::from_bytes(&bytes).is_none());
}

#[test]
fn channel_volumes_fallback() {
    let obj = Object {
        type_: SpaTypes::ObjectParamProps.as_raw(),
        id: ParamType::Props.as_raw(),
        properties: vec![
            Property::new(
                SPA_PROP_CHANNEL_VOLUMES,
                Value::ValueArray(ValueArray::Float(vec![0.25, 0.5, 0.1])),
            ),
            Property::new(SPA_PROP_MUTE, Value::Bool(false)),
        ],
    };
    let bytes = PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(obj))
        .unwrap()
        .0
        .into_inner();
    let pod = Pod::from_bytes(&bytes).unwrap();
    assert_eq!(parse_volume_pod(pod), Some((0.5, false)));
}
