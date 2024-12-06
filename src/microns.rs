#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
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

impl TryFrom<f32> for Microns {
    type Error = &'static str;
    fn try_from(f: f32) -> Result<Self, Self::Error> {
        if f.is_nan() {
            Err("NaN")
        } else {
            Ok(Microns((f * 1000.0).trunc() as i32))
        }
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
        Ok(Microns::try_from(f)?)
    }
}

impl std::ops::Sub for Microns {
    type Output = Microns;
    fn sub(self, rhs: Microns) -> Microns {
        Microns(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::Add for Microns {
    type Output = Microns;
    fn add(self, rhs: Microns) -> Microns {
        Microns(self.0.saturating_add(rhs.0))
    }
}

pub trait Vec5 {
    fn x(&self) -> Microns;
    fn y(&self) -> Microns;
    fn z(&self) -> Microns;
    fn e(&self) -> Microns;
    fn f(&self) -> Microns;
}

impl Vec5 for [Microns; 5] {
    fn x(&self) -> Microns {
        self[0]
    }
    fn y(&self) -> Microns {
        self[1]
    }
    fn z(&self) -> Microns {
        self[2]
    }
    fn e(&self) -> Microns {
        self[3]
    }
    fn f(&self) -> Microns {
        self[4]
    }
}
