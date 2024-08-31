use wasm_ast::node::{BinOpType, CmpOpType, LoadType, StoreType, UnOpType};

pub trait IntoName {
	#[must_use]
	fn into_name(self) -> &'static str;
}

impl IntoName for LoadType {
	fn into_name(self) -> &'static str {
		match self {
			Self::I32 => "i32",
			Self::I64 => "i64",
			Self::F32 => "f32",
			Self::F64 => "f64",
			Self::I32_I8 => "i32_i8",
			Self::I32_U8 => "i32_u8",
			Self::I32_I16 => "i32_i16",
			Self::I32_U16 => "i32_u16",
			Self::I64_I8 => "i64_i8",
			Self::I64_U8 => "i64_u8",
			Self::I64_I16 => "i64_i16",
			Self::I64_U16 => "i64_u16",
			Self::I64_I32 => "i64_i32",
			Self::I64_U32 => "i64_u32",
		}
	}
}

impl IntoName for StoreType {
	fn into_name(self) -> &'static str {
		match self {
			Self::I32 => "i32",
			Self::I64 => "i64",
			Self::F32 => "f32",
			Self::F64 => "f64",
			Self::I32_N8 => "i32_n8",
			Self::I32_N16 => "i32_n16",
			Self::I64_N8 => "i64_n8",
			Self::I64_N16 => "i64_n16",
			Self::I64_N32 => "i64_n32",
		}
	}
}

pub trait IntoNameTuple {
	#[must_use]
	fn into_name_tuple(self) -> (&'static str, &'static str);
}

impl IntoNameTuple for UnOpType {
	fn into_name_tuple(self) -> (&'static str, &'static str) {
		match self {
			Self::Clz_I32 => ("bit", "countlz"),
			Self::Ctz_I32 => ("bit", "countrz"),
			Self::Popcnt_I32 => ("rt_popcnt", "i32"),
			Self::Clz_I64 => ("rt_clz", "i64"),
			Self::Ctz_I64 => ("rt_ctz", "i64"),
			Self::Popcnt_I64 => ("rt_popcnt", "i64"),
			Self::Abs_F32 => ("math", "abs"),
			Self::Neg_F32 => ("rt_neg", "f32"),
			Self::Ceil_F32 => ("math", "ceil"),
			Self::Floor_F32 => ("math", "floor"),
			Self::Truncate_F32 => ("rt_truncate", "f32"),
			Self::Nearest_F32 => ("rt_nearest", "f32"),
			Self::Sqrt_F32 => ("math", "sqrt"),
			Self::Abs_F64 => ("math", "abs"),
			Self::Neg_F64 => ("rt_neg", "f64"),
			Self::Ceil_F64 => ("math", "ceil"),
			Self::Floor_F64 => ("math", "floor"),
			Self::Truncate_F64 => ("rt_truncate", "f64"),
			Self::Nearest_F64 => ("rt_nearest", "f64"),
			Self::Sqrt_F64 => ("math", "sqrt"),
			Self::Wrap_I32_I64 => ("rt_wrap", "i32_i64"),
			Self::Truncate_I32_F32 => ("rt_truncate", "i32_f64"),
			Self::Truncate_I32_F64 => ("rt_truncate", "i32_f64"),
			Self::Truncate_U32_F32 => ("rt_truncate", "f64"),
			Self::Truncate_U32_F64 => ("rt_truncate", "f64"),
			Self::Truncate_I64_F32 => ("rt_truncate", "i64_f64"),
			Self::Truncate_I64_F64 => ("rt_truncate", "i64_f64"),
			Self::Truncate_U64_F32 => ("rt_truncate", "u64_f64"),
			Self::Truncate_U64_F64 => ("rt_truncate", "u64_f64"),
			Self::Saturate_I32_F32 => ("rt_saturate", "i32_f32"),
			Self::Saturate_I32_F64 => ("rt_saturate", "i32_f64"),
			Self::Saturate_U32_F32 => ("rt_saturate", "u32_f32"),
			Self::Saturate_U32_F64 => ("rt_saturate", "u32_f64"),
			Self::Saturate_I64_F32 => ("rt_saturate", "i64_f32"),
			Self::Saturate_I64_F64 => ("rt_saturate", "i64_f64"),
			Self::Saturate_U64_F32 => ("rt_saturate", "u64_f32"),
			Self::Saturate_U64_F64 => ("rt_saturate", "u64_f64"),
			Self::Extend_I32_N8 => ("rt_extend", "i32_n8"),
			Self::Extend_I32_N16 => ("rt_extend", "i32_n16"),
			Self::Extend_I64_N8 => ("rt_extend", "i64_n8"),
			Self::Extend_I64_N16 => ("rt_extend", "i64_n16"),
			Self::Extend_I64_N32 => ("rt_extend", "i64_n32"),
			Self::Extend_I64_I32 => ("rt_extend", "i64_i32"),
			Self::Extend_I64_U32 => ("rt_extend", "i64_u32"),
			Self::Convert_F32_I32 => ("rt_convert", "f64_i32"),
			Self::Convert_F32_U32 => ("no", "op"),
			Self::Convert_F32_I64 => ("rt_convert", "f64_i64"),
			Self::Convert_F32_U64 => ("rt_convert", "f64_u64"),
			Self::Demote_F32_F64 => ("no", "op"),
			Self::Convert_F64_I32 => ("rt_convert", "f64_i32"),
			Self::Convert_F64_U32 => ("no", "op"),
			Self::Convert_F64_I64 => ("rt_convert", "f64_i64"),
			Self::Convert_F64_U64 => ("rt_convert", "f64_u64"),
			Self::Promote_F64_F32 => ("no", "op"),
			Self::Reinterpret_I32_F32 => ("rt_reinterpret", "i32_f32"),
			Self::Reinterpret_I64_F64 => ("rt_reinterpret", "i64_f64"),
			Self::Reinterpret_F32_I32 => ("rt_reinterpret", "f32_i32"),
			Self::Reinterpret_F64_I64 => ("rt_reinterpret", "f64_i64"),
		}
	}
}

impl IntoNameTuple for BinOpType {
	fn into_name_tuple(self) -> (&'static str, &'static str) {
		match self {
			Self::Add_I32 => ("rt_add", "i32"),
			Self::Sub_I32 => ("rt_sub", "i32"),
			Self::Mul_I32 => ("rt_mul", "i32"),
			Self::DivS_I32 => ("rt_div", "i32"),
			Self::DivU_I32 => ("rt_div", "u32"),
			Self::RemS_I32 => ("rt_rem", "i32"),
			Self::RemU_I32 => ("rt_rem", "u32"),
			Self::And_I32 => ("bit", "and"),
			Self::Or_I32 => ("bit", "or"),
			Self::Xor_I32 => ("bit", "xor"),
			Self::Shl_I32 => ("rt_shl", "i32"),
			Self::ShrS_I32 => ("rt_shr", "i32"),
			Self::ShrU_I32 => ("rt_shr", "u32"),
			Self::Rotl_I32 => ("rt_rotl", "i32"),
			Self::Rotr_I32 => ("rt_rotr", "i32"),
			Self::Add_I64 => ("rt_add", "i64"),
			Self::Sub_I64 => ("rt_sub", "i64"),
			Self::Mul_I64 => ("rt_mul", "i64"),
			Self::DivS_I64 => ("rt_div", "i64"),
			Self::DivU_I64 => ("rt_div", "u64"),
			Self::RemS_I64 => ("rt_rem", "i64"),
			Self::RemU_I64 => ("rt_rem", "u64"),
			Self::And_I64 => ("rt_bit_and", "i64"),
			Self::Or_I64 => ("rt_bit_or", "i64"),
			Self::Xor_I64 => ("rt_bit_xor", "i64"),
			Self::Shl_I64 => ("rt_shl", "i64"),
			Self::ShrS_I64 => ("rt_shr", "i64"),
			Self::ShrU_I64 => ("rt_shr", "u64"),
			Self::Rotl_I64 => ("rt_rotl", "i64"),
			Self::Rotr_I64 => ("rt_rotr", "i64"),
			Self::Add_F32 => ("rt_add", "f32"),
			Self::Sub_F32 => ("rt_sub", "f32"),
			Self::Mul_F32 => ("rt_mul", "f32"),
			Self::Div_F32 => ("rt_div", "f32"),
			Self::Min_F32 => ("rt_min", "f32"),
			Self::Max_F32 => ("rt_max", "f32"),
			Self::Copysign_F32 => ("rt_copysign", "f32"),
			Self::Add_F64 => ("rt_add", "f64"),
			Self::Sub_F64 => ("rt_sub", "f64"),
			Self::Mul_F64 => ("rt_mul", "f64"),
			Self::Div_F64 => ("rt_div", "f64"),
			Self::Min_F64 => ("rt_min", "f64"),
			Self::Max_F64 => ("rt_max", "f64"),
			Self::Copysign_F64 => ("rt_copysign", "f64"),
		}
	}
}

impl IntoNameTuple for CmpOpType {
	fn into_name_tuple(self) -> (&'static str, &'static str) {
		match self {
			Self::Eq_I32 => ("rt_eq", "i32"),
			Self::Ne_I32 => ("rt_ne", "i32"),
			Self::LtS_I32 => ("rt_lt", "i32"),
			Self::LtU_I32 => ("rt_lt", "u32"),
			Self::GtS_I32 => ("rt_gt", "i32"),
			Self::GtU_I32 => ("rt_gt", "u32"),
			Self::LeS_I32 => ("rt_le", "i32"),
			Self::LeU_I32 => ("rt_le", "u32"),
			Self::GeS_I32 => ("rt_ge", "i32"),
			Self::GeU_I32 => ("rt_ge", "u32"),
			Self::Eq_I64 => ("rt_eq", "i64"),
			Self::Ne_I64 => ("rt_ne", "i64"),
			Self::LtS_I64 => ("rt_lt", "i64"),
			Self::LtU_I64 => ("rt_lt", "u64"),
			Self::GtS_I64 => ("rt_gt", "i64"),
			Self::GtU_I64 => ("rt_gt", "u64"),
			Self::LeS_I64 => ("rt_le", "i64"),
			Self::LeU_I64 => ("rt_le", "u64"),
			Self::GeS_I64 => ("rt_ge", "i64"),
			Self::GeU_I64 => ("rt_ge", "u64"),
			Self::Eq_F32 => ("rt_eq", "f32"),
			Self::Ne_F32 => ("rt_ne", "f32"),
			Self::Lt_F32 => ("rt_lt", "f32"),
			Self::Gt_F32 => ("rt_gt", "f32"),
			Self::Le_F32 => ("rt_le", "f32"),
			Self::Ge_F32 => ("rt_ge", "f32"),
			Self::Eq_F64 => ("rt_eq", "f64"),
			Self::Ne_F64 => ("rt_ne", "f64"),
			Self::Lt_F64 => ("rt_lt", "f64"),
			Self::Gt_F64 => ("rt_gt", "f64"),
			Self::Le_F64 => ("rt_le", "f64"),
			Self::Ge_F64 => ("rt_ge", "f64"),
		}
	}
}

pub trait TryIntoSymbol {
	#[must_use]
	fn try_into_symbol(self) -> Option<&'static str>;
}

impl TryIntoSymbol for BinOpType {
	fn try_into_symbol(self) -> Option<&'static str> {
		let result = match self {
			Self::Add_F32 | Self::Add_F64 => "+",
			Self::Sub_F32 | Self::Sub_F64 => "-",
			Self::Mul_F32 | Self::Mul_F64 => "*",
			Self::Div_F32 | Self::Div_F64 => "/",
			Self::RemU_I32 => "%",
			_ => return None,
		};

		Some(result)
	}
}

impl TryIntoSymbol for CmpOpType {
	fn try_into_symbol(self) -> Option<&'static str> {
		let result = match self {
			Self::Eq_I32 | Self::Eq_F32 | Self::Eq_F64 => "==",
			Self::Ne_I32 | Self::Ne_F32 | Self::Ne_F64 => "~=",
			Self::LtU_I32 | Self::Lt_F32 | Self::Lt_F64 => "<",
			Self::GtU_I32 | Self::Gt_F32 | Self::Gt_F64 => ">",
			Self::LeU_I32 | Self::Le_F32 | Self::Le_F64 => "<=",
			Self::GeU_I32 | Self::Ge_F32 | Self::Ge_F64 => ">=",
			_ => return None,
		};

		Some(result)
	}
}
