#![allow(dead_code)]
use crate::GCodeModel;

/// Any command that should be automatically parsed and stored as
/// print metadata should be stored here, even if it is the same
/// for every flavor.
struct FlavorProfile {
    pub flavor: Flavor,
    temp_command: &'static str,
    bed_temp_command: &'static str,
}

enum Flavor {
    Marlin,
    RepRapFirmware,
    Prusa,
    Bambu,
    Klipper
}
impl TryFrom<&GCodeModel> for Flavor {
    type Error = &'static str;
    fn try_from(_gcode: &GCodeModel) -> Result<Self, Self::Error> {
        let flavor = Err("Unknown flavor");
        flavor
    }
}

impl Flavor {
    fn temp_command(&self) -> &'static str {
        match self {
            Flavor::Marlin => "M104",
            Flavor::RepRapFirmware => "M104",
            Flavor::Prusa => "M104",
            Flavor::Bambu => "M104",
            Flavor::Klipper => "M104",
        }
    }
    fn bed_temp_command(&self) -> &'static str {
        match self {
            Flavor::Marlin => "M140",
            Flavor::RepRapFirmware => "M140",
            Flavor::Prusa => "M140",
            Flavor::Bambu => "M140",
            Flavor::Klipper => "M140",
        }
    }
    fn fan_speed_command(&self) -> &'static str {
        match self {
            Flavor::Marlin => "M106",
            Flavor::RepRapFirmware => "M106",
            Flavor::Prusa => "M106",
            Flavor::Bambu => "M106",
            Flavor::Klipper => "M106",
        }
    }
}

struct Meta {
    pub flavor: Flavor,
    pub printer: String,
    pub material: String,
    pub nozzle: f32,
    pub layer_height: f32,
    pub temperature: f32,
    pub fan_speed: f32,
    pub bed_temperature: f32,
}

fn preprint() {}