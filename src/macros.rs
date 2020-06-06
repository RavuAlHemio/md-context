#![macro_use]

macro_rules! accessor {
    ($prop:ident, $tp:ty) => {
        pub fn $prop(&self) -> &$tp {
            &self.$prop
        }
    };
}

macro_rules! accessor_opt {
    ($prop:ident, $tp:ty) => {
        pub fn $prop(&self) -> Option<&$tp> {
            if let Some(val) = &self.$prop {
                Some(val)
            } else {
                None
            }
        }
    };
}

macro_rules! accessor_and_mut {
    ($prop:ident, $prop_mut:ident, $tp:ty) => {
        accessor!($prop, $tp);

        pub fn $prop_mut(&mut self) -> &mut $tp {
            &mut self.$prop
        }
    };
}
