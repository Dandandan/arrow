# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.

context("Expressions")

test_that("Can create an expression", {
  expect_is(build_array_expression(">", Array$create(1:5), 4), "array_expression")
})

test_that("as.vector(array_expression)", {
  expect_equal(as.vector(build_array_expression(">", Array$create(1:5), 4)), c(FALSE, FALSE, FALSE, FALSE, TRUE))
})

test_that("array_expression print method", {
  expect_output(
    print(build_array_expression(">", Array$create(1:5), 4)),
    # Not ideal but it is informative
    "greater(<Array>, 4)",
    fixed = TRUE
  )
})

test_that("array_refs", {
  tab <- Table$create(a = 1:5)
  ex <- build_array_expression(">", array_expression("array_ref", field_name = "a"), 4)
  expect_is(ex, "array_expression")
  expect_identical(ex$args[[1]]$args$field_name, "a")
  expect_identical(find_array_refs(ex), "a")
  out <- eval_array_expression(ex, tab)
  expect_is(out, "ChunkedArray")
  expect_equal(as.vector(out), c(FALSE, FALSE, FALSE, FALSE, TRUE))
})

test_that("C++ expressions", {
  f <- Expression$field_ref("f")
  expect_identical(f$field_name, "f")
  g <- Expression$field_ref("g")
  date <- Expression$scalar(as.Date("2020-01-15"))
  ts <- Expression$scalar(as.POSIXct("2020-01-17 11:11:11"))
  i64 <- Expression$scalar(bit64::as.integer64(42))
  time <- Expression$scalar(hms::hms(56, 34, 12))

  expect_is(f == g, "Expression")
  expect_is(f == 4, "Expression")
  expect_is(f == "", "Expression")
  expect_is(f == NULL, "Expression")
  expect_is(f == date, "Expression")
  expect_is(f == i64, "Expression")
  expect_is(f == time, "Expression")
  # can't seem to make this work right now because of R Ops.method dispatch
  # expect_is(f == as.Date("2020-01-15"), "Expression")
  expect_is(f == ts, "Expression")
  expect_is(f <= 2L, "Expression")
  expect_is(f != FALSE, "Expression")
  expect_is(f > 4, "Expression")
  expect_is(f < 4 & f > 2, "Expression")
  expect_is(f < 4 | f > 2, "Expression")
  expect_is(!(f < 4), "Expression")
  expect_output(
    print(f > 4),
    'Expression\n(f > 4)',
    fixed = TRUE
  )
  # Interprets that as a list type
  expect_is(f == c(1L, 2L), "Expression")
})

test_that("Can create an expression", {
  a <- Array$create(as.numeric(1:5))
  expr <- array_expression("cast", a, options = list(to_type = int32()))
  expect_is(expr, "array_expression")
  expect_equal(eval_array_expression(expr), Array$create(1:5))

  b <- Array$create(0.5:4.5)
  bad_expr <- array_expression("cast", b, options = list(to_type = int32()))
  expect_is(bad_expr, "array_expression")
  expect_error(
    eval_array_expression(bad_expr),
    "Invalid: Float value .* was truncated converting"
  )
  expr <- array_expression("cast", b, options = list(to_type = int32(), allow_float_truncate = TRUE))
  expect_is(expr, "array_expression")
  expect_equal(eval_array_expression(expr), Array$create(0:4))
})
