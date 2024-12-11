use crate::Microns;

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
