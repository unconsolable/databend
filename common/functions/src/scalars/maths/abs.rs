// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;

use common_datavalues::prelude::*;
use common_exception::ErrorCode;
use common_exception::Result;

use crate::scalars::function::Function;
use crate::scalars::function_factory::FunctionDescription;
use crate::scalars::function_factory::FunctionFeatures;
use crate::scalars::Monotonicity;

#[derive(Clone)]
pub struct AbsFunction {
    _display_name: String,
}

impl AbsFunction {
    pub fn try_create(display_name: &str) -> Result<Box<dyn Function>> {
        Ok(Box::new(AbsFunction {
            _display_name: display_name.to_string(),
        }))
    }

    pub fn desc() -> FunctionDescription {
        FunctionDescription::creator(Box::new(Self::try_create))
            .features(FunctionFeatures::default().deterministic().monotonicity())
    }
}

macro_rules! impl_abs_function {
    ($column:expr, $type:ident, $cast_type:expr) => {{
        let mut series = $column.column().to_minimal_array()?;

        // coerce String to Float
        if *$column.data_type() == DataType::String {
            series = series.cast_with_type(&DataType::Float64)?;
        }

        let primitive_array = series.$type()?;
        let column: DataColumn = primitive_array
            .apply_cast_numeric(|v| v.abs())
            .cast_with_type(&$cast_type)?
            .into();
        Ok(column.resize_constant($column.column().len()))
    }};
}

impl Function for AbsFunction {
    fn name(&self) -> &str {
        "abs"
    }

    fn num_arguments(&self) -> usize {
        1
    }

    fn return_type(&self, args: &[DataType]) -> Result<DataType> {
        if args[0].is_numeric() || args[0] == DataType::String || args[0] == DataType::Null {
            Ok(match &args[0] {
                DataType::Int8 => DataType::UInt8,
                DataType::Int16 => DataType::UInt16,
                DataType::Int32 => DataType::UInt32,
                DataType::Int64 => DataType::UInt64,
                DataType::String => DataType::Float64,
                dt => dt.clone(),
            })
        } else {
            Err(ErrorCode::IllegalDataType(format!(
                "Expected numeric types, but got {}",
                args[0]
            )))
        }
    }

    fn nullable(&self, _input_schema: &DataSchema) -> Result<bool> {
        Ok(false)
    }

    fn eval(&self, columns: &DataColumnsWithField, _input_rows: usize) -> Result<DataColumn> {
        match columns[0].data_type() {
            DataType::Int8 => impl_abs_function!(columns[0], i8, DataType::UInt8),
            DataType::Int16 => impl_abs_function!(columns[0], i16, DataType::UInt16),
            DataType::Int32 => impl_abs_function!(columns[0], i32, DataType::UInt32),
            DataType::Int64 => impl_abs_function!(columns[0], i64, DataType::UInt16),
            DataType::Float32 => impl_abs_function!(columns[0], f32, DataType::Float32),
            DataType::Float64 => impl_abs_function!(columns[0], f64, DataType::Float64),
            DataType::String => impl_abs_function!(columns[0], f64, DataType::Float64),
            DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
                Ok(columns[0].column().clone())
            }
            DataType::Null => Ok(columns[0].column().clone()),
            _ => unreachable!(),
        }
    }

    fn get_monotonicity(&self, args: &[Monotonicity]) -> Result<Monotonicity> {
        // for constant value, just return clone
        if args[0].is_constant {
            return Ok(args[0].clone());
        }

        // if either left boundary or right boundary is unknown, we don't known the monotonicity
        if args[0].left.is_none() || args[0].right.is_none() {
            return Ok(Monotonicity::default());
        }

        match args[0].compare_with_zero()? {
            1 => {
                // the range is >= 0, abs function do nothing
                Ok(Monotonicity {
                    is_monotonic: true,
                    is_positive: args[0].is_positive,
                    is_constant: false,
                    left: None,
                    right: None,
                })
            }
            -1 => {
                // the range is <= 0, abs function flip the is_positive
                Ok(Monotonicity {
                    is_monotonic: true,
                    is_positive: !args[0].is_positive,
                    is_constant: false,
                    left: None,
                    right: None,
                })
            }
            _ => Ok(Monotonicity::default()),
        }
    }
}

impl fmt::Display for AbsFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ABS")
    }
}
