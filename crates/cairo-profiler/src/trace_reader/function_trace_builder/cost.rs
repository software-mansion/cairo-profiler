use cairo_lang_sierra::extensions::circuit::CircuitInfo;
use cairo_lang_sierra::extensions::gas::CostTokenType;
use cairo_lang_sierra::ids::ConcreteTypeId;
use cairo_lang_sierra::program::StatementIdx;
use cairo_lang_sierra_ap_change::core_libfunc_ap_change::InvocationApChangeInfoProvider;
use cairo_lang_sierra_gas::core_libfunc_cost::InvocationCostInfoProvider;
use cairo_lang_sierra_to_casm::circuit::CircuitsInfo;
use cairo_lang_sierra_to_casm::metadata::Metadata;
use cairo_lang_sierra_type_size::TypeSizeMap;
use cairo_lang_utils::casts::IntoOrPanic;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use serde::{Deserialize, Serialize};

pub struct ProfilerInvocationInfo<'a> {
    pub type_sizes: &'a TypeSizeMap,
    pub circuits_info: &'a CircuitsInfo,
    pub metadata: &'a Metadata,
    pub idx: StatementIdx,
}

impl InvocationCostInfoProvider for ProfilerInvocationInfo<'_> {
    fn type_size(&self, ty: &ConcreteTypeId) -> usize {
        self.type_sizes[ty].into_or_panic()
    }

    fn token_usages(&self, token_type: CostTokenType) -> usize {
        InvocationApChangeInfoProvider::token_usages(self, token_type)
    }

    fn ap_change_var_value(&self) -> usize {
        self.metadata
            .ap_change_info
            .variable_values
            .get(&self.idx)
            .copied()
            .unwrap_or_default()
    }

    fn circuit_info(&self, ty: &ConcreteTypeId) -> &CircuitInfo {
        self.circuits_info.circuits.get(ty).unwrap()
    }
}

impl InvocationApChangeInfoProvider for ProfilerInvocationInfo<'_> {
    fn type_size(&self, ty: &ConcreteTypeId) -> usize {
        self.type_sizes[ty].into_or_panic()
    }

    fn token_usages(&self, token_type: CostTokenType) -> usize {
        usize::try_from(
            self.metadata
                .gas_info
                .variable_values
                .get(&(self.idx, token_type))
                .copied()
                .unwrap_or(0),
        )
        .unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CostEntry {
    pub pedersen: i64,
    pub poseidon: i64,
    pub bitwise: i64,
    pub ec_op: i64,
    pub add_mod: i64,
    pub mul_mod: i64,
    #[serde(rename = "const")]
    pub konst: i64,
}

impl CostEntry {
    pub fn from_map(map: &OrderedHashMap<CostTokenType, i64>) -> Self {
        Self {
            pedersen: *map.get(&CostTokenType::Pedersen).unwrap_or(&0),
            poseidon: *map.get(&CostTokenType::Poseidon).unwrap_or(&0),
            bitwise: *map.get(&CostTokenType::Bitwise).unwrap_or(&0),
            ec_op: *map.get(&CostTokenType::EcOp).unwrap_or(&0),
            add_mod: *map.get(&CostTokenType::AddMod).unwrap_or(&0),
            mul_mod: *map.get(&CostTokenType::MulMod).unwrap_or(&0),
            konst: *map.get(&CostTokenType::Const).unwrap_or(&0),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (CostTokenType, i64)> + '_ {
        [
            (CostTokenType::Const, self.konst),
            (CostTokenType::Bitwise, self.bitwise),
            (CostTokenType::Pedersen, self.pedersen),
            (CostTokenType::Poseidon, self.poseidon),
            (CostTokenType::EcOp, self.ec_op),
            (CostTokenType::AddMod, self.add_mod),
            (CostTokenType::MulMod, self.mul_mod),
        ]
        .into_iter()
    }
}
