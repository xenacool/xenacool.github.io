#[cfg(test)]
mod tests {
    use crate::*;
    use hexx::Hex;
    use npc_engine_utils::GlobalDomain;
    use proptest::prelude::*;
    use std::collections::HashMap;

    include!("tests_part1.rs");
    include!("tests_part2.rs");
    include!("tests_part3.rs");
}
