#![macro_use]

macro_rules! accessor {
    ($prop:ident, $tp:ty) => {
        pub fn $prop(&self) -> &$tp {
            &self.$prop
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
