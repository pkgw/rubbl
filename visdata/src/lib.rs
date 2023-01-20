// Copyright 2017-2020 Peter Williams
// Licensed under the MIT License.

//! Working with interferometric visibility data.
//!
//! API currently maps quasi-directly onto the MIRIAD data model since those are
//! the files I'm working with.

use anyhow::Error;

/// A "feed pol(arization)" is the polarization component sampled by a
/// particular receptor on an radio antenna.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FeedPol {
    X,
    Y,
    R,
    L,
}

/// A "vis(ibility) pol(arization)" is the polarization sampled by the
/// cross-correlation of the voltages of two radio receptors.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VisPol {
    XX,
    XY,
    YX,
    YY,
    RR,
    RL,
    LR,
    LL,
    I,
    Q,
    U,
    V,
}

impl VisPol {
    pub fn feedpol1(self) -> FeedPol {
        match self {
            VisPol::XX | VisPol::XY => FeedPol::X,
            VisPol::YX | VisPol::YY => FeedPol::Y,
            VisPol::RR | VisPol::RL => FeedPol::R,
            VisPol::LR | VisPol::LL => FeedPol::L,
            VisPol::I | VisPol::Q | VisPol::U | VisPol::V => {
                panic!("cannot convert Stokes VisPol into FeedPol")
            }
        }
    }

    pub fn feedpol2(self) -> FeedPol {
        match self {
            VisPol::XX | VisPol::YX => FeedPol::X,
            VisPol::XY | VisPol::YY => FeedPol::Y,
            VisPol::RR | VisPol::LR => FeedPol::R,
            VisPol::RL | VisPol::LL => FeedPol::L,
            VisPol::I | VisPol::Q | VisPol::U | VisPol::V => {
                panic!("cannot convert Stokes VisPol into FeedPol")
            }
        }
    }
}

type AntNum = u16;

/// An "antpol" specifies a voltage stream fed into a correlator: the
/// combination of a particular antenna and a particular feed on that
/// antenna.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AntPol {
    pub ant: AntNum,
    pub pol: FeedPol,
}

impl AntPol {
    pub fn new(ant: AntNum, pol: FeedPol) -> Self {
        AntPol { ant: ant, pol: pol }
    }
}

/// A "basepol" specifies a visibility output from a correlator: the data
/// stream associated with the correlation of two input antpols.
///
/// TODO: specify what the ordering means in terms of conjugation?
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BasePol {
    pub ant1: AntNum,
    pub ant2: AntNum,
    pub pol: VisPol,
}

impl BasePol {
    pub fn new(ant1: AntNum, ant2: AntNum, pol: VisPol) -> Self {
        BasePol {
            ant1: ant1,
            ant2: ant2,
            pol: pol,
        }
    }

    pub fn antpol1(self) -> AntPol {
        AntPol {
            ant: self.ant1,
            pol: self.pol.feedpol1(),
        }
    }

    pub fn antpol2(self) -> AntPol {
        AntPol {
            ant: self.ant2,
            pol: self.pol.feedpol2(),
        }
    }
}

pub trait VisStream {
    fn next(&mut self) -> Result<bool, Error>;

    fn basepol(&self) -> BasePol;
}
