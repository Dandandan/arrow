// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License

//! Optimizer rule to switch build and probe order of hash join
//! based on statistics of a `TableProvider`. If the number of
//! rows of both sources is known, the order can be switched
//! for a faster hash join.

use crate::logical_plan::LogicalPlan;
use crate::optimizer::optimizer::OptimizerRule;
use crate::{error::Result, prelude::JoinType};

use super::utils;

/// BuildProbeOrder reorders the build and probe phase of
/// hash joins. This uses the amount of rows that a datasource has.
/// The rule optimizes the order such that the left (build) side of the join
/// is the smallest.
/// If the information is not available, the order stays the same,
/// so that it could be optimized manually in a query.
pub struct HashBuildProbeOrder {}

// Gets exact number of rows, if known by the statistics of the underlying
fn get_num_rows(logical_plan: &LogicalPlan) -> Option<usize> {
    match logical_plan {
        LogicalPlan::Projection { input, .. } => get_num_rows(input),
        LogicalPlan::Sort { input, .. } => get_num_rows(input),
        LogicalPlan::TableScan { source, .. } => source.statistics().num_rows,
        LogicalPlan::EmptyRelation {
            produce_one_row, ..
        } => {
            if *produce_one_row {
                Some(1)
            } else {
                Some(0)
            }
        }
        LogicalPlan::Limit { n: limit, input } => {
            let num_rows_input = get_num_rows(input);
            num_rows_input.map(|rows| std::cmp::min(*limit, rows))
        }
        _ => None,
    }
}

// Finds out whether to swap left vs right order based on statistics
fn should_swap_join_order(left: &LogicalPlan, right: &LogicalPlan) -> bool {
    let left_rows = get_num_rows(left);
    let right_rows = get_num_rows(right);
    println!("LEFT-rows: {:?}", left_rows);
    println!("RIGHT-rows: {:?}", right_rows);

    match (left_rows, right_rows) {
        (Some(l), Some(r)) => l > r,
        _ => false,
    }
}

impl OptimizerRule for HashBuildProbeOrder {
    fn name(&self) -> &str {
        "hash_build_probe_order"
    }

    fn optimize(&mut self, plan: &LogicalPlan) -> Result<LogicalPlan> {
        match plan {
            // Main optimization rule, swaps order of left and right
            // based on number of rows in each table
            LogicalPlan::Join {
                left,
                right,
                on,
                join_type,
                schema,
            } => {
                if should_swap_join_order(left, right) {
                    // swap
                    Ok(LogicalPlan::Join {
                        left: right.clone(),
                        right: left.clone(),
                        on: on
                            .iter()
                            .map(|(l, r)| (r.to_string(), l.to_string()))
                            .collect(),
                        join_type: swap_join_type(*join_type),
                        schema: schema.clone(),
                    })
                } else {
                    Ok(LogicalPlan::Join {
                        left: left.clone(),
                        right: right.clone(),
                        on: on.clone(),
                        join_type: *join_type,
                        schema: schema.clone(),
                    })
                }
            }
            // Rest: recurse into plan, apply optimization where possible
            LogicalPlan::Projection { .. }
            | LogicalPlan::Aggregate { .. }
            | LogicalPlan::TableScan { .. }
            | LogicalPlan::Limit { .. }
            | LogicalPlan::Filter { .. }
            | LogicalPlan::EmptyRelation { .. }
            | LogicalPlan::Sort { .. }
            | LogicalPlan::CreateExternalTable { .. }
            | LogicalPlan::Explain { .. }
            | LogicalPlan::Extension { .. } => {
                let expr = utils::expressions(plan);

                // apply the optimization to all inputs of the plan
                let inputs = utils::inputs(plan);
                let new_inputs = inputs
                    .iter()
                    .map(|plan| self.optimize(plan))
                    .collect::<Result<Vec<_>>>()?;

                utils::from_plan(plan, &expr, &new_inputs)
            }
        }
    }
}

impl HashBuildProbeOrder {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {}
    }
}

fn swap_join_type(join_type: JoinType) -> JoinType {
    match join_type {
        JoinType::Inner => JoinType::Inner,
        JoinType::Left => JoinType::Right,
        JoinType::Right => JoinType::Left,
    }
}
