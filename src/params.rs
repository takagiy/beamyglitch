use num_enum::TryFromPrimitive;
use vst::{plugin::PluginParameters, util::AtomicFloat};

pub(crate) struct BeamyGlitchParams {
    pub attack: AtomicFloat,
    pub release: AtomicFloat,
}

impl BeamyGlitchParams {
    fn new() -> Self {
        BeamyGlitchParams {
            attack: AtomicFloat::new(0.003),
            release: AtomicFloat::new(0.02),
        }
    }
}

impl Default for BeamyGlitchParams {
    fn default() -> Self {
        BeamyGlitchParams::new()
    }
}

#[derive(TryFromPrimitive)]
#[repr(i32)]
enum Parameter {
    Attack,
    Release,
}

impl PluginParameters for BeamyGlitchParams {
    fn get_parameter_label(&self, index: i32) -> String {
        match Parameter::try_from(index) {
            Ok(p) => match p {
                Parameter::Attack | Parameter::Release => "ms".to_owned(),
            },
            _ => "".to_owned(),
        }
    }

    fn get_parameter_text(&self, index: i32) -> String {
        match Parameter::try_from(index) {
            Ok(p) => match p {
                Parameter::Attack => format_float(self.attack.get() * 1000.),
                Parameter::Release => format_float(self.release.get() * 1000.),
            },
            _ => "".to_owned(),
        }
    }

    fn get_parameter_name(&self, index: i32) -> String {
        match Parameter::try_from(index) {
            Ok(p) => match p {
                Parameter::Attack => "attack".to_owned(),
                Parameter::Release => "release".to_owned(),
            },
            _ => "".to_owned(),
        }
    }

    fn get_parameter(&self, index: i32) -> f32 {
        match Parameter::try_from(index) {
            Ok(p) => match p {
                Parameter::Attack => self.attack.get(),
                Parameter::Release => self.release.get(),
            },
            _ => 0.,
        }
    }

    fn set_parameter(&self, index: i32, value: f32) {
        if let Ok(p) = Parameter::try_from(index) {
            match p {
                Parameter::Attack => self.attack.set(value),
                Parameter::Release => self.release.set(value),
            }
        }
    }

    fn can_be_automated(&self, _index: i32) -> bool {
        true
    }

    fn string_to_parameter(&self, index: i32, text: String) -> bool {
        match text.parse::<f32>() {
            Ok(value) => match Parameter::try_from(index) {
                Ok(p) => {
                    match p {
                        Parameter::Attack => self.attack.set(value / 1000.),
                        Parameter::Release => self.release.set(value / 1000.),
                    };
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

fn format_float(value: f32) -> String {
    format!("{:.2}", value)
}
