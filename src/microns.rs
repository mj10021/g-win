#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Microns(pub i32);

impl Microns {
    pub const ZERO: Microns = Microns(0);
    pub const MIN: Microns = Microns(i32::MIN);

    pub fn abs(&self) -> Self {
        Microns(self.0.abs())
    }
}

impl From<f32> for Microns {
    fn from(f: f32) -> Self {
        Microns((f * 1000.0).trunc() as i32)
    }
}

impl From<Microns> for f32 {
    fn from(m: Microns) -> f32 {
        m.0 as f32 / 1000.0
    }
}

impl std::str::FromStr for Microns {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = s.parse::<f32>().map_err(|_| "unable to parse float")?;
        Ok(Microns::from(f))
    }
}

impl std::ops::Sub for Microns {
    type Output = Microns;
    fn sub(self, rhs: Microns) -> Microns {
        Microns(self.0 - rhs.0)
    }
}

impl std::ops::Add for Microns {
    type Output = Microns;
    fn add(self, rhs: Microns) -> Microns {
        Microns(self.0 + rhs.0)
    }
}

impl std::ops::Div for Microns {
    type Output = Microns;
    fn div(self, rhs: Microns) -> Microns {
        Microns(self.0 / rhs.0)
    }
}

impl std::ops::Mul for Microns {
    type Output = Microns;
    fn mul(self, rhs: Microns) -> Microns {
        Microns(self.0 * rhs.0)
    }
}

impl std::fmt::Display for Microns {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0 as f32)
    }
}
