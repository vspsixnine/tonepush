//! Which pedal a tone is for, and whether the pedal connected plays it
//! (docs/design/library-workflow-2026-10-02, 16 "Device markers").
//!
//! Every tone row says which pedal it is for in the same two words: the
//! family (HX or PRO), then the model (Stomp, Effects, Helix LT) or, for a
//! StompStation PRO, the firmware its chain needs (2.x when the preset has a
//! chain layout, 1.5 when it has none). The library learns it three ways: it
//! records the pedal a tone was kept from; for a tone kept before that, it
//! reads the document; a TonePush tone carries the device the site lists.

/// A family of pedals that share a preset format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Family {
    /// Line 6's HX and Helix.
    Hx,
    /// Sonulab's StompStation PRO.
    Pro,
}

impl Family {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Family::Hx => "HX",
            Family::Pro => "PRO",
        }
    }
}

/// What a tone is for: its family, then its model or, for a PRO, the
/// firmware generation its chain needs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Marker {
    pub family: Family,
    /// "Stomp", "Stomp XL", "Effects", "Helix", "Helix LT", "Helix Floor" or
    /// "Helix Rack"; "2.x" or "1.5" for a StompStation PRO.
    pub model: String,
}

/// A PRO tone whose preset carries a chain layout: firmware 2.x reads it.
pub(crate) const PRO_ROUTED: &str = "2.x";
/// A PRO tone saved on firmware 1.5, whose blocks sit in a fixed order.
pub(crate) const PRO_FIXED: &str = "1.5";

impl Marker {
    pub(crate) fn hx(model: &str) -> Marker {
        Marker {
            family: Family::Hx,
            model: model.to_owned(),
        }
    }

    pub(crate) fn pro(generation: &str) -> Marker {
        Marker {
            family: Family::Pro,
            model: generation.to_owned(),
        }
    }

    /// The marker of a pedal, by the name it gives itself or TonePush lists
    /// it under, and for a PRO the firmware it runs.
    pub(crate) fn of_device(name: &str, firmware: &str) -> Option<Marker> {
        let name = name.trim();
        if name.to_ascii_lowercase().contains("stompstation") {
            return Some(Marker::pro(match release(firmware) {
                Some((major, _)) if major < 2 => PRO_FIXED,
                _ => PRO_ROUTED,
            }));
        }
        let model = match name {
            "HX Stomp" => "Stomp",
            "HX Stomp XL" => "Stomp XL",
            "HX Effects" => "Effects",
            "Helix" | "Helix Floor" | "Helix LT" | "Helix Rack" => name,
            _ => return None,
        };
        Some(Marker::hx(model))
    }

    /// The pedal itself, as a sentence or TonePush names it: "HX Stomp",
    /// "Helix LT", "StompStation PRO".
    pub(crate) fn device_name(&self) -> String {
        match self.family {
            Family::Pro => "StompStation PRO".to_owned(),
            Family::Hx if self.model.starts_with("Helix") => self.model.clone(),
            Family::Hx => format!("HX {}", self.model),
        }
    }

    /// The device TonePush lists a tone under when it is published. Its
    /// catalog calls a Helix Floor "Helix".
    pub(crate) fn catalog_device(&self) -> String {
        match self.model.as_str() {
            "Helix Floor" => "Helix".to_owned(),
            _ => self.device_name(),
        }
    }

    /// "an HX Stomp", "a StompStation PRO": the pedal with its article.
    pub(crate) fn with_article(&self) -> String {
        let name = self.device_name();
        let article = if name.starts_with('H') && !name.starts_with("Helix") {
            "an"
        } else {
            "a"
        };
        format!("{article} {name}")
    }
}

/// What an HX document says about the pedal it was made on.
///
/// The endpoints carry no model of their own in the device's document, but
/// each family's input is built differently: a Helix has two DSPs and so two
/// signal paths, and an HX Effects input has no noise gate, so its input
/// slot holds no values. Everything else is the HX Stomp's, which an XL reads
/// too.
pub(crate) fn of_hx_document(preset: &hx_proto::Preset) -> Marker {
    use hx_proto::preset::Kind;
    let layout = preset.layout();
    if layout
        .paths
        .iter()
        .filter(|path| path.input.is_some())
        .count()
        >= 2
    {
        return Marker::hx("Helix");
    }
    let input = preset
        .slots
        .iter()
        .find(|slot| slot.kind == Kind::Input)
        .map(|slot| slot.values.len());
    if input == Some(0) {
        return Marker::hx("Effects");
    }
    Marker::hx("Stomp")
}

/// What a `.hlx` file says: it names the device it was saved for.
pub(crate) fn of_hlx(document: &serde_json::Value) -> Option<Marker> {
    let device = document
        .get("data")
        .and_then(|data| data.get("device"))
        .and_then(serde_json::Value::as_u64)?;
    let profile = hx_proto::PROFILES
        .iter()
        .find(|profile| u64::from(profile.device_id) == device)?;
    Marker::of_device(profile.name, "")
}

/// What a StompStation PRO preset says: a chain layout means firmware 2.x.
pub(crate) fn of_pro_document(bytes: &[u8]) -> Option<Marker> {
    let preset = voidx_proto::Preset::parse(bytes).ok()?;
    let routed = preset
        .records()
        .iter()
        .any(|record| record.subject() == voidx_proto::router::PATH);
    Some(Marker::pro(if routed { PRO_ROUTED } else { PRO_FIXED }))
}

/// A firmware version's release: its major and minor numbers. Patch
/// releases are treated alike, so 1.5.10 and 1.5.12 are both 1.5.
///
/// The StompStation PRO writes three numbers (2.0.10). Line 6 writes two
/// with the patch as the last digit of the second (3.81 is 3.8, patch 1), so
/// a two-digit second number is read as minor then patch.
pub(crate) fn release(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.trim().parse().ok()?;
    let second = parts.next()?.trim();
    let third = parts.next();
    let minor = if third.is_none() && second.len() == 2 {
        second.get(..1)?.parse().ok()?
    } else {
        second.parse().ok()?
    };
    Some((major, minor))
}

/// The pedal connected, as a tone's marker is judged against it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pedal {
    /// Its own name: "HX Stomp", "StompStation PRO".
    pub name: String,
    /// The firmware it runs, as it reports it.
    pub firmware: String,
}

impl Pedal {
    pub(crate) fn marker(&self) -> Option<Marker> {
        Marker::of_device(&self.name, &self.firmware)
    }
}

/// Why the pedal connected cannot play a tone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The tone is for the other family.
    Family,
    /// The tone is for another model of the same family.
    Model,
    /// A PRO tone with a chain layout, on a PRO on firmware 1.5.
    NeedsRouting,
    /// Made on a newer firmware release than the pedal runs.
    Firmware { made: String, has: String },
}

/// Whether the pedal connected plays a tone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Plays,
    /// A 1.5 PRO tone on 2.x: the pedal plays it with its default chain.
    PlaysWithDefaultChain,
    Refused(Refusal),
}

impl Verdict {
    pub(crate) fn plays(&self) -> bool {
        !matches!(self, Verdict::Refused(_))
    }
}

/// Whether a pedal of this marker reads presets made for that one. The XL
/// reads the Stomp's, and every Helix reads the others' (they share one
/// platform and one preset format); otherwise each model reads only its own.
fn model_reads(pedal: &str, tone: &str) -> bool {
    pedal == tone
        || (pedal == "Stomp XL" && tone == "Stomp")
        || (pedal.starts_with("Helix") && tone.starts_with("Helix"))
}

/// Judge a tone against the pedal connected: the family, the model or the
/// PRO's generation, and the firmware it was made on when that is known.
pub(crate) fn verdict(pedal: &Pedal, tone: &Marker, tone_firmware: &str) -> Verdict {
    let Some(connected) = pedal.marker() else {
        return Verdict::Plays;
    };
    if connected.family != tone.family {
        return Verdict::Refused(Refusal::Family);
    }
    let newer = match (release(tone_firmware), release(&pedal.firmware)) {
        (Some(made), Some(has)) => made > has,
        _ => false,
    };
    match tone.family {
        Family::Hx => {
            if !model_reads(&connected.model, &tone.model) {
                return Verdict::Refused(Refusal::Model);
            }
            if newer {
                return Verdict::Refused(Refusal::Firmware {
                    made: tone_firmware.trim().to_owned(),
                    has: pedal.firmware.trim().to_owned(),
                });
            }
            Verdict::Plays
        }
        Family::Pro => {
            if tone.model == PRO_ROUTED && connected.model == PRO_FIXED {
                return Verdict::Refused(Refusal::NeedsRouting);
            }
            if newer {
                return Verdict::Refused(Refusal::Firmware {
                    made: tone_firmware.trim().to_owned(),
                    has: pedal.firmware.trim().to_owned(),
                });
            }
            if tone.model == PRO_FIXED && connected.model == PRO_ROUTED {
                Verdict::PlaysWithDefaultChain
            } else {
                Verdict::Plays
            }
        }
    }
}

/// Why a tone cannot play, in one sentence that names it: "Velvet Drive is
/// a StompStation PRO tone. The HX Stomp cannot play it."
pub(crate) fn refusal_words(refusal: &Refusal, name: &str, tone: &Marker, pedal: &Pedal) -> String {
    match refusal {
        Refusal::Family | Refusal::Model => format!(
            "{name} is {} tone. The {} cannot play it.",
            tone.with_article(),
            pedal.name.trim()
        ),
        Refusal::NeedsRouting => format!(
            "{name} needs firmware 2.x; this {} is on {}.",
            pedal.name.trim(),
            pedal.firmware.trim()
        ),
        Refusal::Firmware { made, has } => format!(
            "{name} was made on firmware {made}; this {} has {has}.",
            pedal.name.trim()
        ),
    }
}

/// The same, as short as a hover can be: "The HX Stomp cannot play a
/// StompStation PRO tone".
pub(crate) fn refusal_hint(refusal: &Refusal, tone: &Marker, pedal: &Pedal) -> String {
    match refusal {
        Refusal::Family | Refusal::Model => format!(
            "The {} cannot play {} tone",
            pedal.name.trim(),
            tone.with_article()
        ),
        Refusal::NeedsRouting => format!(
            "Needs firmware 2.x; this {} is on {}",
            pedal.name.trim(),
            pedal.firmware.trim()
        ),
        Refusal::Firmware { made, has } => format!(
            "Made on firmware {made}; this {} has {has}",
            pedal.name.trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pedal(name: &str, firmware: &str) -> Pedal {
        Pedal {
            name: name.to_owned(),
            firmware: firmware.to_owned(),
        }
    }

    #[test]
    fn a_pedal_is_named_by_its_family_then_its_model() {
        assert_eq!(
            Marker::of_device("HX Stomp", "3.80"),
            Some(Marker::hx("Stomp"))
        );
        assert_eq!(
            Marker::of_device("HX Stomp XL", ""),
            Some(Marker::hx("Stomp XL"))
        );
        assert_eq!(
            Marker::of_device("HX Effects", ""),
            Some(Marker::hx("Effects"))
        );
        assert_eq!(
            Marker::of_device("Helix LT", ""),
            Some(Marker::hx("Helix LT"))
        );
        assert_eq!(
            Marker::of_device("StompStation PRO", "2.0.10"),
            Some(Marker::pro("2.x"))
        );
        assert_eq!(
            Marker::of_device("StompStation PRO", "1.5.12"),
            Some(Marker::pro("1.5"))
        );
        assert_eq!(Marker::of_device("POD Go", ""), None);
        assert_eq!(Marker::hx("Effects").device_name(), "HX Effects");
        assert_eq!(Marker::hx("Helix LT").device_name(), "Helix LT");
        assert_eq!(Marker::hx("Helix Floor").catalog_device(), "Helix");
        assert_eq!(Marker::pro("2.x").with_article(), "a StompStation PRO");
        assert_eq!(Marker::hx("Stomp").with_article(), "an HX Stomp");
        assert_eq!(Marker::hx("Helix").with_article(), "a Helix");
    }

    /// Patch releases are alike: 1.5.10 is 1.5.12, and 3.81 is 3.80.
    #[test]
    fn firmware_is_judged_by_major_and_minor() {
        assert_eq!(release("1.5.12"), Some((1, 5)));
        assert_eq!(release("1.5.10"), Some((1, 5)));
        assert_eq!(release("2.0.10"), Some((2, 0)));
        assert_eq!(release("2.2.6"), Some((2, 2)));
        assert_eq!(release("3.80"), Some((3, 8)));
        assert_eq!(release("3.81"), Some((3, 8)));
        assert_eq!(release("3.70"), Some((3, 7)));
        assert_eq!(release(""), None);
        assert_eq!(release("new"), None);
    }

    /// Sheet 16's table: what each pedal plays, by TonePush's rules.
    #[test]
    fn each_pedal_plays_what_its_family_and_model_read() {
        let stomp = pedal("HX Stomp", "3.80");
        let xl = pedal("HX Stomp XL", "3.80");
        let effects = pedal("HX Effects", "3.80");
        let floor = pedal("Helix Floor", "3.80");
        assert_eq!(verdict(&stomp, &Marker::hx("Stomp"), ""), Verdict::Plays);
        assert_eq!(
            verdict(&stomp, &Marker::hx("Stomp XL"), ""),
            Verdict::Refused(Refusal::Model)
        );
        assert_eq!(verdict(&xl, &Marker::hx("Stomp"), ""), Verdict::Plays);
        assert_eq!(verdict(&xl, &Marker::hx("Stomp XL"), ""), Verdict::Plays);
        assert_eq!(
            verdict(&stomp, &Marker::hx("Effects"), ""),
            Verdict::Refused(Refusal::Model)
        );
        assert_eq!(
            verdict(&effects, &Marker::hx("Effects"), ""),
            Verdict::Plays
        );
        assert_eq!(verdict(&floor, &Marker::hx("Helix"), ""), Verdict::Plays);
        assert_eq!(verdict(&floor, &Marker::hx("Helix LT"), ""), Verdict::Plays);
        assert_eq!(
            verdict(&stomp, &Marker::pro("2.x"), ""),
            Verdict::Refused(Refusal::Family)
        );
    }

    #[test]
    fn a_pro_reads_its_own_generation_and_the_older_one() {
        let routed = pedal("StompStation PRO", "2.0.10");
        let fixed = pedal("StompStation PRO", "1.5.12");
        assert_eq!(verdict(&routed, &Marker::pro("2.x"), ""), Verdict::Plays);
        assert_eq!(
            verdict(&routed, &Marker::pro("1.5"), ""),
            Verdict::PlaysWithDefaultChain
        );
        assert_eq!(verdict(&fixed, &Marker::pro("1.5"), ""), Verdict::Plays);
        assert_eq!(
            verdict(&fixed, &Marker::pro("2.x"), ""),
            Verdict::Refused(Refusal::NeedsRouting)
        );
        assert_eq!(
            verdict(&routed, &Marker::hx("Stomp"), ""),
            Verdict::Refused(Refusal::Family)
        );
    }

    #[test]
    fn a_tone_made_on_newer_firmware_is_refused_and_a_patch_is_not() {
        let stomp = pedal("HX Stomp", "3.70");
        assert_eq!(
            verdict(&stomp, &Marker::hx("Stomp"), "3.80"),
            Verdict::Refused(Refusal::Firmware {
                made: "3.80".into(),
                has: "3.70".into()
            })
        );
        assert_eq!(
            verdict(&stomp, &Marker::hx("Stomp"), "3.71"),
            Verdict::Plays
        );
        assert_eq!(
            verdict(&stomp, &Marker::hx("Stomp"), "3.50"),
            Verdict::Plays
        );
        let pro = pedal("StompStation PRO", "2.0.10");
        assert_eq!(verdict(&pro, &Marker::pro("2.x"), "2.0.12"), Verdict::Plays);
        assert!(!verdict(&pro, &Marker::pro("2.x"), "2.2.6").plays());
        let fixed = pedal("StompStation PRO", "1.5.10");
        assert_eq!(
            verdict(&fixed, &Marker::pro("1.5"), "1.5.12"),
            Verdict::Plays
        );
    }

    #[test]
    fn a_refusal_says_why_in_one_sentence() {
        let stomp = pedal("HX Stomp", "3.80");
        assert_eq!(
            refusal_words(
                &Refusal::Family,
                "Velvet Drive",
                &Marker::pro("2.x"),
                &stomp
            ),
            "Velvet Drive is a StompStation PRO tone. The HX Stomp cannot play it."
        );
        assert_eq!(
            refusal_words(
                &Refusal::Model,
                "Pedalboard Wash",
                &Marker::hx("Effects"),
                &stomp
            ),
            "Pedalboard Wash is an HX Effects tone. The HX Stomp cannot play it."
        );
        let fixed = pedal("StompStation PRO", "1.5.12");
        assert_eq!(
            refusal_words(
                &Refusal::NeedsRouting,
                "Shimmer Lead",
                &Marker::pro("2.x"),
                &fixed
            ),
            "Shimmer Lead needs firmware 2.x; this StompStation PRO is on 1.5.12."
        );
        assert_eq!(
            refusal_hint(&Refusal::Family, &Marker::pro("2.x"), &stomp),
            "The HX Stomp cannot play a StompStation PRO tone"
        );
    }

    #[test]
    fn an_hx_document_says_which_family_made_it() {
        let stomp = include_bytes!("../../hx-proto/tests/fixtures/gen-04-full-rig.hxpreset");
        let preset = hx_proto::Preset::parse(stomp).expect("the fixture parses");
        assert_eq!(of_hx_document(&preset), Marker::hx("Stomp"));
        assert_eq!(preset.firmware().as_deref(), Some("3.80"));
    }

    #[test]
    fn an_hlx_names_its_device() {
        let document = serde_json::json!({ "data": { "device": 0x0021_0005u32 } });
        assert_eq!(of_hlx(&document), Some(Marker::hx("Effects")));
        assert_eq!(of_hlx(&serde_json::json!({})), None);
    }

    /// Firmware 2.x saves the chain as a preset's first record; 1.5.12
    /// saves none.
    #[test]
    fn a_pro_preset_with_a_chain_layout_needs_2x() {
        let routed = b"root\\app\\router:{\"value\":[[\"root\\\\app\\\\gate\",\"p\",\"root\\\\app\\\\amp\"]]}\r\n\
root\\app\\gate\\on_off:{\"value\":\"ON\"}\r\n";
        let fixed = b"root\\app\\gate\\on_off:{\"value\":\"ON\"}\r\n";
        assert_eq!(of_pro_document(routed), Some(Marker::pro("2.x")));
        assert_eq!(of_pro_document(fixed), Some(Marker::pro("1.5")));
        assert_eq!(of_pro_document(b"\0not a preset"), None);
    }
}
