use std::{collections::BTreeSet, io::Result, ops::Range};

use parity_wasm::elements::{
	External, ImportCountType, Instruction, Internal, Module, NameSection, ResizableLimits,
};

use wasm_ast::{
	builder::{Builder, TypeInfo},
	node::{
		AnyBinOp, AnyCmpOp, AnyLoad, AnyStore, AnyUnOp, Backward, Br, BrIf, BrTable, Call,
		CallIndirect, Else, Expression, Forward, Function, GetGlobal, GetLocal, If, Memorize,
		MemoryGrow, MemorySize, Recall, Return, Select, SetGlobal, SetLocal, Statement, Value,
	},
	writer::{Transpiler, Writer},
};

use super::analyzer::{localize, memory};

fn aux_internal_index(internal: Internal) -> u32 {
	match internal {
		Internal::Function(v) | Internal::Table(v) | Internal::Memory(v) | Internal::Global(v) => v,
	}
}

fn new_limit_max(limits: &ResizableLimits) -> String {
	match limits.maximum() {
		Some(v) => v.to_string(),
		None => "0xFFFF".to_string(),
	}
}

fn write_table_init(limit: &ResizableLimits, w: Writer) -> Result<()> {
	let a = limit.initial();
	let b = new_limit_max(limit);

	write!(w, "{{ min = {}, max = {}, data = {{}} }}", a, b)
}

fn write_memory_init(limit: &ResizableLimits, w: Writer) -> Result<()> {
	let a = limit.initial();
	let b = new_limit_max(limit);

	write!(w, "rt.allocator.new({}, {})", a, b)
}

fn write_func_name(wasm: &Module, index: u32, offset: u32, w: Writer) -> Result<()> {
	let opt = wasm
		.names_section()
		.and_then(NameSection::functions)
		.and_then(|v| v.names().get(index));

	write!(w, "FUNC_LIST")?;

	if let Some(name) = opt {
		write!(w, "--[[{}]]", name)?;
	}

	write!(w, "[{}] =", index + offset)
}

fn write_in_order(prefix: &str, len: usize, w: Writer) -> Result<()> {
	if len == 0 {
		return Ok(());
	}

	write!(w, "{}_{}", prefix, 0)?;
	(1..len).try_for_each(|i| write!(w, ", {}_{}", prefix, i))
}

fn write_f32(f: f32, w: Writer) -> Result<()> {
	let sign = if f.is_sign_negative() { "-" } else { "" };

	if f.is_infinite() {
		write!(w, "{}math.huge ", sign)
	} else if f.is_nan() {
		write!(w, "{}0/0 ", sign)
	} else {
		write!(w, "{:e} ", f)
	}
}

fn write_f64(f: f64, w: Writer) -> Result<()> {
	let sign = if f.is_sign_negative() { "-" } else { "" };

	if f.is_infinite() {
		write!(w, "{}math.huge ", sign)
	} else if f.is_nan() {
		write!(w, "{}0/0 ", sign)
	} else {
		write!(w, "{:e} ", f)
	}
}

fn write_list(name: &str, len: usize, w: Writer) -> Result<()> {
	let len = len.saturating_sub(1);

	write!(w, "local {} = table.create({})", name, len)
}

fn write_parameter_list(func: &Function, w: Writer) -> Result<()> {
	write!(w, "function(")?;
	write_in_order("param", func.num_param, w)?;
	write!(w, ")")
}

fn write_result_list(range: Range<usize>, w: Writer) -> Result<()> {
	if range.is_empty() {
		return Ok(());
	}

	range.clone().try_for_each(|i| {
		if i != range.start {
			write!(w, ", ")?;
		}

		write!(w, "reg_{}", i)
	})?;

	write!(w, " = ")
}

fn write_variable_list(func: &Function, w: Writer) -> Result<()> {
	for data in &func.local_data {
		write!(w, "local ")?;
		write_in_order("loc", data.count().try_into().unwrap(), w)?;
		write!(w, " = ")?;

		for i in 0..data.count() {
			if i != 0 {
				write!(w, ", ")?;
			}

			write!(w, "ZERO_{} ", data.value_type())?;
		}
	}

	if func.num_stack != 0 {
		write!(w, "local ")?;
		write_in_order("reg", func.num_stack, w)?;
		write!(w, " ")?;
	}

	Ok(())
}

fn write_expression(code: &[Instruction], w: Writer) -> Result<()> {
	// FIXME: Badly generated WASM will produce the wrong constant.
	for inst in code {
		let result = match *inst {
			Instruction::I32Const(v) => write!(w, "{} ", v),
			Instruction::I64Const(v) => write!(w, "{} ", v),
			Instruction::F32Const(v) => write_f32(f32::from_bits(v), w),
			Instruction::F64Const(v) => write_f64(f64::from_bits(v), w),
			Instruction::GetGlobal(i) => write!(w, "GLOBAL_LIST[{}].value ", i),
			_ => {
				continue;
			}
		};

		return result;
	}

	write!(w, "error(\"mundane expression\")")
}

fn br_target(level: usize, in_loop: bool, w: Writer) -> Result<()> {
	write!(w, "if desired then ")?;
	write!(w, "if desired == {} then ", level)?;
	write!(w, "desired = nil ")?;

	if in_loop {
		write!(w, "continue ")?;
	}

	write!(w, "end ")?;
	write!(w, "break ")?;
	write!(w, "end ")
}

#[derive(PartialEq, Eq)]
enum Label {
	Forward,
	Backward,
	If,
}

#[derive(Default)]
struct Visitor {
	label_list: Vec<Label>,
	num_param: usize,
}

impl Visitor {
	fn write_br_gadget(&self, rem: usize, w: Writer) -> Result<()> {
		match self.label_list.last() {
			Some(Label::Forward | Label::If) => br_target(rem, false, w),
			Some(Label::Backward) => br_target(rem, true, w),
			None => Ok(()),
		}
	}

	fn push_label(&mut self, label: Label) -> usize {
		self.label_list.push(label);

		self.label_list.len() - 1
	}

	fn pop_label(&mut self) {
		self.label_list.pop().unwrap();
	}
}

trait Driver {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()>;
}

impl Driver for Recall {
	fn visit(&self, _: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "reg_{} ", self.var)
	}
}

impl Driver for Select {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "(")?;
		self.cond.visit(v, w)?;
		write!(w, "~= 0 and ")?;
		self.a.visit(v, w)?;
		write!(w, "or ")?;
		self.b.visit(v, w)?;
		write!(w, ")")
	}
}

fn write_variable(var: usize, v: &Visitor, w: Writer) -> Result<()> {
	if let Some(rem) = var.checked_sub(v.num_param) {
		write!(w, "loc_{} ", rem)
	} else {
		write!(w, "param_{} ", var)
	}
}

impl Driver for GetLocal {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_variable(self.var, v, w)
	}
}

impl Driver for GetGlobal {
	fn visit(&self, _: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "GLOBAL_LIST[{}].value ", self.var)
	}
}

impl Driver for AnyLoad {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "load_{}(memory_at_0, ", self.op.as_name())?;
		self.pointer.visit(v, w)?;
		write!(w, "+ {})", self.offset)
	}
}

impl Driver for MemorySize {
	fn visit(&self, _: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "memory_at_{}.min ", self.memory)
	}
}

impl Driver for MemoryGrow {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "rt.allocator.grow(memory_at_{}, ", self.memory)?;
		self.value.visit(v, w)?;
		write!(w, ")")
	}
}

impl Driver for Value {
	fn visit(&self, _: &mut Visitor, w: Writer) -> Result<()> {
		match self {
			Self::I32(i) => write!(w, "{} ", i),
			Self::I64(i) => write!(w, "{} ", i),
			Self::F32(f) => write_f32(*f, w),
			Self::F64(f) => write_f64(*f, w),
		}
	}
}

impl Driver for AnyUnOp {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		let (a, b) = self.op.as_name();

		write!(w, "{}_{}(", a, b)?;
		self.rhs.visit(v, w)?;
		write!(w, ")")
	}
}

fn write_bin_op(bin_op: &AnyBinOp, v: &mut Visitor, w: Writer) -> Result<()> {
	let op = bin_op.op.as_operator().unwrap();

	write!(w, "(")?;
	bin_op.lhs.visit(v, w)?;
	write!(w, "{} ", op)?;
	bin_op.rhs.visit(v, w)?;
	write!(w, ")")
}

fn write_bin_op_call(bin_op: &AnyBinOp, v: &mut Visitor, w: Writer) -> Result<()> {
	let (a, b) = bin_op.op.as_name();

	write!(w, "{}_{}(", a, b)?;
	bin_op.lhs.visit(v, w)?;
	write!(w, ", ")?;
	bin_op.rhs.visit(v, w)?;
	write!(w, ")")
}

impl Driver for AnyBinOp {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		if self.op.as_operator().is_some() {
			write_bin_op(self, v, w)
		} else {
			write_bin_op_call(self, v, w)
		}
	}
}

impl Driver for AnyCmpOp {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		let (a, b) = self.op.as_name();

		write!(w, "{}_{}(", a, b)?;
		self.lhs.visit(v, w)?;
		write!(w, ", ")?;
		self.rhs.visit(v, w)?;
		write!(w, ")")
	}
}

fn write_expr_list(list: &[Expression], v: &mut Visitor, w: Writer) -> Result<()> {
	list.iter().enumerate().try_for_each(|(i, e)| {
		if i != 0 {
			write!(w, ", ")?;
		}

		e.visit(v, w)
	})
}

impl Driver for Expression {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		match self {
			Self::Recall(e) => e.visit(v, w),
			Self::Select(e) => e.visit(v, w),
			Self::GetLocal(e) => e.visit(v, w),
			Self::GetGlobal(e) => e.visit(v, w),
			Self::AnyLoad(e) => e.visit(v, w),
			Self::MemorySize(e) => e.visit(v, w),
			Self::MemoryGrow(e) => e.visit(v, w),
			Self::Value(e) => e.visit(v, w),
			Self::AnyUnOp(e) => e.visit(v, w),
			Self::AnyBinOp(e) => e.visit(v, w),
			Self::AnyCmpOp(e) => e.visit(v, w),
		}
	}
}

impl Driver for Memorize {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "reg_{} = ", self.var)?;
		self.value.visit(v, w)
	}
}

impl Driver for Forward {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		let rem = v.push_label(Label::Forward);

		write!(w, "while true do ")?;

		self.body.iter().try_for_each(|s| s.visit(v, w))?;

		write!(w, "break ")?;
		write!(w, "end ")?;

		v.pop_label();
		v.write_br_gadget(rem, w)
	}
}

impl Driver for Backward {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		let rem = v.push_label(Label::Backward);

		write!(w, "while true do ")?;

		self.body.iter().try_for_each(|s| s.visit(v, w))?;

		write!(w, "break ")?;
		write!(w, "end ")?;

		v.pop_label();
		v.write_br_gadget(rem, w)
	}
}

impl Driver for Else {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "else ")?;

		self.body.iter().try_for_each(|s| s.visit(v, w))
	}
}

impl Driver for If {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		let rem = v.push_label(Label::If);

		write!(w, "while true do ")?;
		write!(w, "if ")?;
		self.cond.visit(v, w)?;
		write!(w, "~= 0 then ")?;

		self.truthy.iter().try_for_each(|s| s.visit(v, w))?;

		if let Some(s) = &self.falsey {
			s.visit(v, w)?;
		}

		write!(w, "end ")?;
		write!(w, "break ")?;
		write!(w, "end ")?;

		v.pop_label();
		v.write_br_gadget(rem, w)
	}
}

fn write_br_at(up: usize, v: &Visitor, w: Writer) -> Result<()> {
	write!(w, "do ")?;

	if up == 0 {
		if let Some(&Label::Backward) = v.label_list.last() {
			write!(w, "continue ")?;
		} else {
			write!(w, "break ")?;
		}
	} else {
		let level = v.label_list.len() - 1 - up;

		write!(w, "desired = {} ", level)?;
		write!(w, "break ")?;
	}

	write!(w, "end ")
}

impl Driver for Br {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_br_at(self.target, v, w)
	}
}

impl Driver for BrIf {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "if ")?;
		self.cond.visit(v, w)?;
		write!(w, "~= 0 then ")?;

		write_br_at(self.target, v, w)?;

		write!(w, "end ")
	}
}

impl Driver for BrTable {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "do ")?;
		write!(w, "local temp = {{")?;

		if !self.data.table.is_empty() {
			write!(w, "[0] =")?;

			for d in self.data.table.iter() {
				write!(w, "{}, ", d)?;
			}
		}

		write!(w, "}} ")?;

		write!(w, "desired = temp[")?;
		self.cond.visit(v, w)?;
		write!(w, "] or {} ", self.data.default)?;
		write!(w, "break ")?;
		write!(w, "end ")
	}
}

impl Driver for Return {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "do return ")?;

		write_expr_list(&self.list, v, w)?;

		write!(w, "end ")
	}
}

impl Driver for Call {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_result_list(self.result.clone(), w)?;

		write!(w, "FUNC_LIST[{}](", self.func)?;

		write_expr_list(&self.param_list, v, w)?;

		write!(w, ")")
	}
}

impl Driver for CallIndirect {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_result_list(self.result.clone(), w)?;

		write!(w, "TABLE_LIST[{}].data[", self.table)?;

		self.index.visit(v, w)?;

		write!(w, "](")?;

		write_expr_list(&self.param_list, v, w)?;

		write!(w, ")")
	}
}

impl Driver for SetLocal {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_variable(self.var, v, w)?;

		write!(w, "= ")?;
		self.value.visit(v, w)
	}
}

impl Driver for SetGlobal {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "GLOBAL_LIST[{}].value = ", self.var)?;
		self.value.visit(v, w)
	}
}

impl Driver for AnyStore {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write!(w, "store_{}(memory_at_0, ", self.op.as_name())?;
		self.pointer.visit(v, w)?;
		write!(w, "+ {}, ", self.offset)?;
		self.value.visit(v, w)?;
		write!(w, ")")
	}
}

impl Driver for Statement {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		match self {
			Statement::Unreachable => write!(w, "error(\"out of code bounds\")"),
			Statement::Memorize(s) => s.visit(v, w),
			Statement::Forward(s) => s.visit(v, w),
			Statement::Backward(s) => s.visit(v, w),
			Statement::If(s) => s.visit(v, w),
			Statement::Br(s) => s.visit(v, w),
			Statement::BrIf(s) => s.visit(v, w),
			Statement::BrTable(s) => s.visit(v, w),
			Statement::Return(s) => s.visit(v, w),
			Statement::Call(s) => s.visit(v, w),
			Statement::CallIndirect(s) => s.visit(v, w),
			Statement::SetLocal(s) => s.visit(v, w),
			Statement::SetGlobal(s) => s.visit(v, w),
			Statement::AnyStore(s) => s.visit(v, w),
		}
	}
}

impl Driver for Function {
	fn visit(&self, v: &mut Visitor, w: Writer) -> Result<()> {
		write_parameter_list(self, w)?;

		for v in memory::visit(self) {
			write!(w, "local memory_at_{0} = MEMORY_LIST[{0}]", v)?;
		}

		write_variable_list(self, w)?;

		v.num_param = self.num_param;
		self.code.visit(v, w)?;

		write!(w, "end ")
	}
}

pub struct Generator<'a> {
	wasm: &'a Module,
	type_info: TypeInfo<'a>,
}

static RUNTIME: &str = include_str!("../runtime/runtime.lua");

impl<'a> Transpiler<'a> for Generator<'a> {
	fn new(wasm: &'a Module) -> Self {
		let type_info = TypeInfo::from_module(wasm);

		Self { wasm, type_info }
	}

	fn runtime(w: Writer) -> Result<()> {
		write!(w, "{}", RUNTIME)
	}

	fn transpile(&self, w: Writer) -> Result<()> {
		write!(w, "local rt = require(script.Runtime)")?;

		let func_list = self.build_func_list();

		Self::gen_localize(&func_list, w)?;

		write!(w, "local ZERO_i32 = 0 ")?;
		write!(w, "local ZERO_i64 = 0 ")?;
		write!(w, "local ZERO_f32 = 0.0 ")?;
		write!(w, "local ZERO_f64 = 0.0 ")?;

		write_list("FUNC_LIST", self.wasm.functions_space(), w)?;
		write_list("TABLE_LIST", self.wasm.table_space(), w)?;
		write_list("MEMORY_LIST", self.wasm.memory_space(), w)?;
		write_list("GLOBAL_LIST", self.wasm.globals_space(), w)?;

		self.gen_func_list(&func_list, w)?;
		self.gen_start_point(w)
	}
}

impl<'a> Generator<'a> {
	fn gen_import_of<T>(&self, w: Writer, lower: &str, cond: T) -> Result<()>
	where
		T: Fn(&External) -> bool,
	{
		let import = match self.wasm.import_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};
		let upper = lower.to_uppercase();

		for (i, v) in import.iter().filter(|v| cond(v.external())).enumerate() {
			let field = v.field();
			let module = v.module();

			write!(w, "{}[{}] = wasm.{}.{}.{} ", upper, i, module, lower, field)?;
		}

		Ok(())
	}

	fn gen_export_of<T>(&self, w: Writer, lower: &str, cond: T) -> Result<()>
	where
		T: Fn(&Internal) -> bool,
	{
		let export = match self.wasm.export_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};
		let upper = lower.to_uppercase();

		write!(w, "{} = {{", lower)?;

		for v in export.iter().filter(|v| cond(v.internal())) {
			let field = v.field();
			let index = aux_internal_index(*v.internal());

			write!(w, "{} = {}[{}],", field, upper, index)?;
		}

		write!(w, "}},")
	}

	fn gen_import_list(&self, w: Writer) -> Result<()> {
		self.gen_import_of(w, "func_list", |v| matches!(v, External::Function(_)))?;
		self.gen_import_of(w, "table_list", |v| matches!(v, External::Table(_)))?;
		self.gen_import_of(w, "memory_list", |v| matches!(v, External::Memory(_)))?;
		self.gen_import_of(w, "global_list", |v| matches!(v, External::Global(_)))
	}

	fn gen_export_list(&self, w: Writer) -> Result<()> {
		self.gen_export_of(w, "func_list", |v| matches!(v, Internal::Function(_)))?;
		self.gen_export_of(w, "table_list", |v| matches!(v, Internal::Table(_)))?;
		self.gen_export_of(w, "memory_list", |v| matches!(v, Internal::Memory(_)))?;
		self.gen_export_of(w, "global_list", |v| matches!(v, Internal::Global(_)))
	}

	fn gen_table_list(&self, w: Writer) -> Result<()> {
		let table = match self.wasm.table_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};
		let offset = self.wasm.import_count(ImportCountType::Table);

		for (i, v) in table.iter().enumerate() {
			let index = i + offset;

			write!(w, "TABLE_LIST[{}] =", index)?;
			write_table_init(v.limits(), w)?;
		}

		Ok(())
	}

	fn gen_memory_list(&self, w: Writer) -> Result<()> {
		let memory = match self.wasm.memory_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};
		let offset = self.wasm.import_count(ImportCountType::Memory);

		for (i, v) in memory.iter().enumerate() {
			let index = i + offset;

			write!(w, "MEMORY_LIST[{}] =", index)?;
			write_memory_init(v.limits(), w)?;
		}

		Ok(())
	}

	fn gen_global_list(&self, w: Writer) -> Result<()> {
		let global = match self.wasm.global_section() {
			Some(v) => v,
			None => return Ok(()),
		};
		let offset = self.wasm.import_count(ImportCountType::Global);

		for (i, v) in global.entries().iter().enumerate() {
			let index = i + offset;

			write!(w, "GLOBAL_LIST[{}] = {{ value =", index)?;

			write_expression(v.init_expr().code(), w)?;

			write!(w, "}}")?;
		}

		Ok(())
	}

	fn gen_element_list(&self, w: Writer) -> Result<()> {
		let element = match self.wasm.elements_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};

		for v in element {
			write!(w, "do ")?;
			write!(w, "local target = TABLE_LIST[{}].data ", v.index())?;
			write!(w, "local offset =")?;

			write_expression(v.offset().as_ref().unwrap().code(), w)?;

			write!(w, "local data = {{")?;

			v.members()
				.iter()
				.try_for_each(|v| write!(w, "FUNC_LIST[{}],", v))?;

			write!(w, "}}")?;

			write!(w, "table.move(data, 1, #data, offset, target)")?;

			write!(w, "end ")?;
		}

		Ok(())
	}

	fn gen_data_list(&self, w: Writer) -> Result<()> {
		let data = match self.wasm.data_section() {
			Some(v) => v.entries(),
			None => return Ok(()),
		};

		for v in data {
			write!(w, "do ")?;
			write!(w, "local target = MEMORY_LIST[{}]", v.index())?;
			write!(w, "local offset =")?;

			write_expression(v.offset().as_ref().unwrap().code(), w)?;

			write!(w, "local data = \"")?;

			v.value()
				.iter()
				.try_for_each(|v| write!(w, "\\x{:02X}", v))?;

			write!(w, "\"")?;

			write!(w, "rt.allocator.init(target, offset, data)")?;

			write!(w, "end ")?;
		}

		Ok(())
	}

	fn gen_start_point(&self, w: Writer) -> Result<()> {
		write!(w, "local function run_init_code()")?;
		self.gen_table_list(w)?;
		self.gen_memory_list(w)?;
		self.gen_global_list(w)?;
		self.gen_element_list(w)?;
		self.gen_data_list(w)?;
		write!(w, "end ")?;

		write!(w, "return function(wasm)")?;
		self.gen_import_list(w)?;
		write!(w, "run_init_code()")?;

		if let Some(start) = self.wasm.start_section() {
			write!(w, "FUNC_LIST[{}]()", start)?;
		}

		write!(w, "return {{")?;
		self.gen_export_list(w)?;
		write!(w, "}} end ")
	}

	fn gen_localize(func_list: &[Function], w: Writer) -> Result<()> {
		let mut loc_set = BTreeSet::new();

		for func in func_list {
			loc_set.extend(localize::visit(func));
		}

		loc_set
			.into_iter()
			.try_for_each(|(a, b)| write!(w, "local {0}_{1} = rt.{0}.{1} ", a, b))
	}

	fn build_func_list(&self) -> Vec<Function> {
		let list = self.wasm.code_section().unwrap().bodies();
		let iter = list.iter().enumerate();

		iter.map(|f| Builder::new(&self.type_info).consume(f.0, f.1))
			.collect()
	}

	fn gen_func_list(&self, func_list: &[Function], w: Writer) -> Result<()> {
		let o = self.type_info.len_ex();

		func_list.iter().enumerate().try_for_each(|(i, v)| {
			write_func_name(self.wasm, i.try_into().unwrap(), o.try_into().unwrap(), w)?;

			v.visit(&mut Visitor::default(), w)
		})
	}
}
