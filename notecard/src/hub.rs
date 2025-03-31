use serde::{Serialize, Deserialize};

pub mod req {

    use super::*;
    use crate::NoteTransaction;

    #[derive(Deserialize, Serialize, defmt::Format)]
    pub struct HubGet {
        pub req: &'static str
    }

    impl Default for HubGet {
        fn default() -> Self {
            Self {
                req: "hub.get"
            }
        }
    }

    impl NoteTransaction for HubGet {
        type NoteResult = res::Hub;
    }

    #[derive(Deserialize, Serialize, defmt::Format)]
    #[serde(rename_all = "lowercase")]
    pub enum HubMode {
        Periodic,
        Continuous,
        Minimum,
        Off,
        DFU,
    }

    #[derive(Deserialize, Serialize, defmt::Format)]
    pub struct HubSet<'a> {
        pub req: &'static str,

        pub product: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub host: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mode: Option<HubMode>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sn: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub outbound: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub duration: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub voutbound: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub inbound: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub vinbound: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub align: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sync: Option<bool>,
    }

    impl <'a> Default for HubSet<'a> {
        fn default() -> Self {
            Self {
                req: "hub.set",
                product: Default::default(),
                host: Default::default(),
                mode: Default::default(),
                sn: Default::default(),
                outbound: Default::default(),
                duration: Default::default(),
                voutbound: Default::default(),
                inbound: Default::default(),
                vinbound: Default::default(),
                align: Default::default(),
                sync: Default::default(),
            }
    }
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}

    #[derive(Deserialize, defmt::Format)]
    pub struct Hub {
        pub device: Option<heapless::String<40>>,
        pub product: Option<heapless::String<120>>,
        pub mode: Option<self::req::HubMode>,
        pub outbound: Option<u32>,
        pub voutbound: Option<f32>,
        pub inbound: Option<u32>,
        pub vinbound: Option<f32>,
        pub host: Option<heapless::String<40>>,
        pub sn: Option<heapless::String<120>>,
        pub sync: Option<bool>,
    }
}
