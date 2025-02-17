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

use std::sync::Arc;

use common_base::tokio;
use common_exception::Result;
use futures::TryStreamExt;

use crate::storages::system::TablesTable;
use crate::storages::Table;
use crate::storages::ToReadDataSourcePlan;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_tables_table() -> Result<()> {
    let ctx = crate::tests::create_query_context()?;
    let table: Arc<dyn Table> = Arc::new(TablesTable::create(1));
    let source_plan = table.read_plan(ctx.clone(), None).await?;

    let stream = table.read(ctx, &source_plan).await?;
    let result = stream.try_collect::<Vec<_>>().await?;
    let block = &result[0];
    assert_eq!(block.num_columns(), 3);

    let expected = vec![
        "+----------+--------------+--------------------+",
        "| database | name         | engine             |",
        "+----------+--------------+--------------------+",
        "| system   | clusters     | SystemClusters     |",
        "| system   | columns      | SystemColumns      |",
        "| system   | configs      | SystemConfigs      |",
        "| system   | contributors | SystemContributors |",
        "| system   | credits      | SystemCredits      |",
        "| system   | databases    | SystemDatabases    |",
        "| system   | functions    | SystemFunctions    |",
        "| system   | metrics      | SystemMetrics      |",
        "| system   | one          | SystemOne          |",
        "| system   | processes    | SystemProcesses    |",
        "| system   | settings     | SystemSettings     |",
        "| system   | tables       | SystemTables       |",
        "| system   | tracing      | SystemTracing      |",
        "| system   | users        | SystemUsers        |",
        "+----------+--------------+--------------------+",
    ];
    common_datablocks::assert_blocks_sorted_eq(expected, result.as_slice());

    Ok(())
}
