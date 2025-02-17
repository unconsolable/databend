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

#[cfg(test)]
mod monotonicity_check {
    use common_datavalues::prelude::DataColumn;
    use common_datavalues::prelude::DataColumnWithField;
    use common_datavalues::DataField;
    use common_datavalues::DataType;
    use common_datavalues::DataValue;
    use common_exception::Result;
    use common_functions::scalars::Monotonicity;
    use common_planners::*;
    use float_cmp::approx_eq;

    use crate::optimizers::MonotonicityCheckVisitor;

    struct Test {
        name: &'static str,
        expr: Expression,
        column: &'static str,
        left: Option<DataColumnWithField>,
        right: Option<DataColumnWithField>,
        expect_mono: Monotonicity,
        error: &'static str,
    }

    fn create_data(d: f64) -> Option<DataColumnWithField> {
        let data_field = DataField::new("x", DataType::Float64, false);
        let data_column = DataColumn::Constant(DataValue::Float64(Some(d)), 1);
        Some(DataColumnWithField::new(data_column, data_field))
    }

    fn extract_data(data_column_field: DataColumnWithField) -> Result<f64> {
        let arr = data_column_field
            .column()
            .to_minimal_array()?
            .cast_with_type(&DataType::Float64)?;
        let val = arr.f64()?.into_iter().next().unwrap();
        Ok(*val.unwrap())
    }

    fn verify_test(t: Test) -> Result<()> {
        let mono =
            match MonotonicityCheckVisitor::check_expression(&t.expr, t.left, t.right, t.column) {
                Ok(mono) => mono,
                Err(e) => {
                    assert_eq!(t.error, e.to_string(), "{}", t.name);
                    return Ok(());
                }
            };

        assert_eq!(
            mono.is_monotonic, t.expect_mono.is_monotonic,
            "{} is_monotonic",
            t.name
        );
        assert_eq!(
            mono.is_constant, t.expect_mono.is_constant,
            "{} is_constant",
            t.name
        );

        if t.expect_mono.is_monotonic {
            assert_eq!(
                mono.is_positive, t.expect_mono.is_positive,
                "{} is_positive",
                t.name
            );
        }

        if t.expect_mono.is_monotonic || t.expect_mono.is_constant {
            let left = mono.left;
            let right = mono.right;

            let expected_left = t.expect_mono.left;
            let expected_right = t.expect_mono.right;

            if expected_left.is_none() {
                assert!(left.is_none(), "{} left", t.name);
            } else {
                assert!(
                    approx_eq!(
                        f64,
                        extract_data(left.unwrap())?,
                        extract_data(expected_left.unwrap())?,
                        epsilon = 0.000001
                    ),
                    "{} left",
                    t.name
                );
            }

            if expected_right.is_none() {
                assert!(right.is_none(), "{} right", t.name);
            } else {
                assert!(
                    approx_eq!(
                        f64,
                        extract_data(right.unwrap())?,
                        extract_data(expected_right.unwrap())?,
                        epsilon = 0.000001
                    ),
                    "{} right",
                    t.name
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_arithmetic_plus_minus() -> Result<()> {
        let test_suite = vec![
            Test {
                name: "f(x) = x + 12",
                expr: Expression::create_binary_expression("+", vec![col("x"), lit(12i32)]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = -x + 12",
                expr: Expression::create_binary_expression("+", vec![
                    Expression::create_unary_expression("-", vec![col("x")]),
                    lit(12i32),
                ]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x,y) = x + y", // multi-variable function is not supported,
                expr: Expression::create_binary_expression("+", vec![col("x"), col("y")]),
                column: "",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: true,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "Code: 6, displayText = expect column name \"x\", get \"y\".",
            },
            Test {
                name: "f(x) = (-x + 12) - x + (1 - x)",
                expr: Expression::create_binary_expression("+", vec![
                    Expression::create_binary_expression("-", vec![
                        // -x + 12
                        Expression::create_binary_expression("+", vec![
                            Expression::create_unary_expression("-", vec![col("x")]),
                            lit(12i32),
                        ]),
                        col("x"),
                    ]),
                    // 1 - x
                    Expression::create_unary_expression("-", vec![lit(1i64), col("x")]),
                ]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = (x + 12) - x + (1 - x)",
                expr: Expression::create_binary_expression("+", vec![
                    Expression::create_binary_expression("-", vec![
                        // x + 12
                        Expression::create_binary_expression("+", vec![col("x"), lit(12i32)]),
                        col("x"),
                    ]),
                    // 1 - x
                    Expression::create_unary_expression("-", vec![lit(1i64), col("x")]),
                ]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
        ];

        for t in test_suite.into_iter() {
            verify_test(t)?;
        }
        Ok(())
    }

    #[test]
    fn test_arithmetic_mul_div() -> Result<()> {
        let test_suite = vec![
            Test {
                name: "f(x) = -5 * x",
                expr: Expression::create_binary_expression("*", vec![lit(-5_i8), col("x")]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = -1/x",
                expr: Expression::create_binary_expression("/", vec![lit(-1_i8), col("x")]),
                column: "x",
                left: create_data(5.0),
                right: create_data(10.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(-0.2),
                    right: create_data(-0.1),
                },
                error: "",
            },
            Test {
                name: "f(x) = x/10",
                expr: Expression::create_binary_expression("/", vec![col("x"), lit(10_i8)]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = x * (x-12) where x in [10-1000]",
                expr: Expression::create_binary_expression("*", vec![
                    col("x"),
                    Expression::create_binary_expression("-", vec![col("x"), lit(12_i64)]),
                ]),
                column: "x",
                left: create_data(10.0),
                right: create_data(1000.0),
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = x * (x-12) where x in [12, 100]",
                expr: Expression::create_binary_expression("*", vec![
                    col("x"),
                    Expression::create_binary_expression("-", vec![col("x"), lit(12_i64)]),
                ]),
                column: "x",
                left: create_data(12.0),
                right: create_data(100.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(0.0),
                    right: create_data(8800.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = x/(1/x) where  x >= 1",
                expr: Expression::create_binary_expression("/", vec![
                    col("x"),
                    Expression::create_binary_expression("/", vec![lit(1_i8), col("x")]),
                ]),
                column: "x",
                left: create_data(1.0),
                right: create_data(2.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(1.0),
                    right: create_data(4.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = -x/(2/(x-2)) where  x in [0-10]",
                expr: Expression::create_binary_expression("/", vec![
                    Expression::create_unary_expression("-", vec![col("x")]),
                    Expression::create_binary_expression("/", vec![
                        lit(2_i8),
                        Expression::create_binary_expression("-", vec![col("x"), lit(2_i8)]),
                    ]),
                ]),
                column: "x",
                left: create_data(0.0),
                right: create_data(10.0),
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = -x/(2/(x-2)) where  x in [4-10]",
                expr: Expression::create_binary_expression("/", vec![
                    Expression::create_unary_expression("-", vec![col("x")]),
                    Expression::create_binary_expression("/", vec![
                        lit(2_i8),
                        Expression::create_binary_expression("-", vec![col("x"), lit(2_i8)]),
                    ]),
                ]),
                column: "x",
                left: create_data(4.0),
                right: create_data(10.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: create_data(-4.0),
                    right: create_data(-40.0),
                },
                error: "",
            },
        ];

        for t in test_suite.into_iter() {
            verify_test(t)?;
        }
        Ok(())
    }

    #[test]
    fn test_abs_function() -> Result<()> {
        let test_suite = vec![
            Test {
                name: "f(x) = abs(x + 12)",
                expr: Expression::create_scalar_function("abs", vec![
                    Expression::create_binary_expression("+", vec![col("x"), lit(12i32)]),
                ]),
                column: "x",
                left: None,
                right: None,
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: true,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = abs(x) where  0 <= x <= 10",
                expr: Expression::create_scalar_function("abs", vec![col("x")]),
                column: "x",
                left: create_data(0.0),
                right: create_data(10.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(0.0),
                    right: create_data(10.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = abs(x) where  -10 <= x <= -2",
                expr: Expression::create_scalar_function("abs", vec![col("x")]),
                column: "x",
                left: create_data(-10.0),
                right: create_data(-2.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: create_data(10.0),
                    right: create_data(2.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = abs(x) where -5 <= x <= 5", // should NOT be monotonic
                expr: Expression::create_scalar_function("abs", vec![col("x")]),
                column: "x",
                left: create_data(-5.0),
                right: create_data(5.0),
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: false,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = abs(x + 12) where -12 <= x <= 1000",
                expr: Expression::create_scalar_function("abs", vec![
                    Expression::create_binary_expression("+", vec![col("x"), lit(12i32)]),
                ]),
                column: "x",
                left: create_data(-12.0),
                right: create_data(1000.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(0.0),
                    right: create_data(1012.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = abs(x + 12) where -14 <=  x <= 20", // should NOT be monotonic
                expr: Expression::create_scalar_function("abs", vec![
                    Expression::create_binary_expression("+", vec![col("x"), lit(12i32)]),
                ]),
                column: "x",
                left: create_data(-14.0),
                right: create_data(20.0),
                expect_mono: Monotonicity {
                    is_monotonic: false,
                    is_positive: true,
                    is_constant: false,
                    left: None,
                    right: None,
                },
                error: "",
            },
            Test {
                name: "f(x) = abs( (x - 7) + (x - 3) ) where 5 <= x <= 100",
                expr: Expression::create_scalar_function("abs", vec![
                    Expression::create_binary_expression("+", vec![
                        Expression::create_binary_expression("-", vec![col("x"), lit(7_i32)]),
                        Expression::create_binary_expression("-", vec![col("x"), lit(3_i32)]),
                    ]),
                ]),
                column: "x",
                left: create_data(5.0),
                right: create_data(100.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: true,
                    is_constant: false,
                    left: create_data(0.0),
                    right: create_data(190.0),
                },
                error: "",
            },
            Test {
                name: "f(x) = abs( (-x + 8) - x) where -100 <= x <= 4",
                expr: Expression::create_scalar_function("abs", vec![
                    Expression::create_binary_expression("-", vec![
                        Expression::create_binary_expression("+", vec![
                            Expression::create_unary_expression("-", vec![col("x")]),
                            lit(8_i64),
                        ]),
                        col("x"),
                    ]),
                ]),
                column: "x",
                left: create_data(-100.0),
                right: create_data(4.0),
                expect_mono: Monotonicity {
                    is_monotonic: true,
                    is_positive: false,
                    is_constant: false,
                    left: create_data(208.0),
                    right: create_data(0.0),
                },
                error: "",
            },
        ];

        for t in test_suite.into_iter() {
            verify_test(t)?;
        }
        Ok(())
    }
}
