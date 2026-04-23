use crate::execution::{
    ChunkPlanningMode, Chunkability, ExecutionMemoryBudget, PartitionOrdering, StageMemoryProfile,
    TraceLocalChunkPlanRecommendation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceLocalChunkPlan {
    pub ordering: PartitionOrdering,
    pub max_active_partitions: usize,
    pub tiles_per_partition: usize,
    pub partition_count: usize,
    pub compatibility_target_bytes: u64,
    pub resident_partition_bytes: u64,
    pub global_worker_workspace_bytes: u64,
    pub estimated_peak_bytes: u64,
}

pub(crate) fn compile_trace_local_chunk_plan(
    total_tiles: usize,
    ordering: PartitionOrdering,
    models: &[StageMemoryProfile],
    budget: &ExecutionMemoryBudget,
    mode: ChunkPlanningMode,
    preferred_partition_count: Option<usize>,
) -> Option<TraceLocalChunkPlan> {
    let total_tiles = total_tiles.max(1);
    let worker_count = budget.worker_count.max(1);
    if models.is_empty() {
        return None;
    }

    if models
        .iter()
        .any(|model| matches!(model.chunkability, Chunkability::FullVolumeOnly))
    {
        return full_volume_chunk_plan(total_tiles, ordering, models, budget);
    }

    let mut best: Option<TraceLocalChunkPlan> = None;
    for partition_count in 1..=total_tiles {
        let max_active_partitions = worker_count.min(partition_count).max(1);
        for active_partitions in 1..=max_active_partitions {
            let Some(candidate) = compile_tile_span_candidate(
                total_tiles,
                ordering,
                models,
                budget,
                partition_count,
                active_partitions,
            ) else {
                continue;
            };
            if better_candidate(
                &candidate,
                best.as_ref(),
                mode,
                preferred_partition_count.unwrap_or(1),
            ) {
                best = Some(candidate);
            }
        }
    }
    best
}

pub(crate) fn recommendation_from_chunk_plan(
    plan: &TraceLocalChunkPlan,
) -> TraceLocalChunkPlanRecommendation {
    TraceLocalChunkPlanRecommendation {
        max_active_partitions: plan.max_active_partitions,
        tiles_per_partition: plan.tiles_per_partition,
        partition_count: plan.partition_count,
        compatibility_target_bytes: plan.compatibility_target_bytes,
        resident_partition_bytes: plan.resident_partition_bytes,
        global_worker_workspace_bytes: plan.global_worker_workspace_bytes,
        estimated_peak_bytes: plan.estimated_peak_bytes,
    }
}

fn full_volume_chunk_plan(
    total_tiles: usize,
    ordering: PartitionOrdering,
    models: &[StageMemoryProfile],
    budget: &ExecutionMemoryBudget,
) -> Option<TraceLocalChunkPlan> {
    let mut peak_bytes = 0u64;
    let mut compatibility_target_bytes = 0u64;
    let mut resident_partition_bytes = 0u64;
    let mut global_worker_workspace_bytes = 0u64;
    for model in models {
        let peak = estimated_peak_bytes(total_tiles, 1, model, budget)?;
        peak_bytes = peak_bytes.max(peak);
        resident_partition_bytes =
            resident_partition_bytes.max(resident_bytes_for_partition(total_tiles, model));
        compatibility_target_bytes = compatibility_target_bytes
            .max(model.primary_tile_bytes.saturating_mul(total_tiles as u64));
        global_worker_workspace_bytes = global_worker_workspace_bytes
            .max(global_worker_workspace_bytes_for_model(model, budget));
    }

    Some(TraceLocalChunkPlan {
        ordering,
        max_active_partitions: 1,
        tiles_per_partition: total_tiles,
        partition_count: 1,
        compatibility_target_bytes: compatibility_target_bytes.max(1),
        resident_partition_bytes,
        global_worker_workspace_bytes,
        estimated_peak_bytes: peak_bytes,
    })
}

fn compile_tile_span_candidate(
    total_tiles: usize,
    ordering: PartitionOrdering,
    models: &[StageMemoryProfile],
    budget: &ExecutionMemoryBudget,
    partition_count: usize,
    active_partitions: usize,
) -> Option<TraceLocalChunkPlan> {
    let partition_count = partition_count.max(1).min(total_tiles);
    let tiles_per_partition = total_tiles.div_ceil(partition_count).max(1);
    let mut peak_bytes = 0u64;
    let mut resident_partition_bytes = 0u64;
    let mut compatibility_target_bytes = 0u64;
    let mut global_worker_workspace_bytes = 0u64;
    for model in models {
        if !model_supports_candidate(model, budget, tiles_per_partition, active_partitions)? {
            return None;
        }
        let peak = estimated_peak_bytes(tiles_per_partition, active_partitions, model, budget)?;
        peak_bytes = peak_bytes.max(peak);
        resident_partition_bytes =
            resident_partition_bytes.max(resident_bytes_for_partition(tiles_per_partition, model));
        compatibility_target_bytes = compatibility_target_bytes.max(
            model
                .primary_tile_bytes
                .saturating_mul(tiles_per_partition as u64),
        );
        global_worker_workspace_bytes = global_worker_workspace_bytes
            .max(global_worker_workspace_bytes_for_model(model, budget));
    }

    Some(TraceLocalChunkPlan {
        ordering,
        max_active_partitions: active_partitions,
        tiles_per_partition,
        partition_count,
        compatibility_target_bytes: compatibility_target_bytes.max(1),
        resident_partition_bytes,
        global_worker_workspace_bytes,
        estimated_peak_bytes: peak_bytes,
    })
}

fn model_supports_candidate(
    model: &StageMemoryProfile,
    budget: &ExecutionMemoryBudget,
    tiles_per_partition: usize,
    active_partitions: usize,
) -> Option<bool> {
    let active_partitions = active_partitions.max(1) as u64;
    let global_worker_workspace_bytes = global_worker_workspace_bytes_for_model(model, budget);
    let partition_bytes =
        resident_bytes_for_partition(tiles_per_partition, model).checked_mul(active_partitions)?;
    let required_without_reserve = budget
        .usable_bytes
        .checked_sub(model.shared_stage_bytes)?
        .checked_sub(global_worker_workspace_bytes)?;
    Some(partition_bytes <= required_without_reserve)
}

fn estimated_peak_bytes(
    tiles_per_partition: usize,
    active_partitions: usize,
    model: &StageMemoryProfile,
    budget: &ExecutionMemoryBudget,
) -> Option<u64> {
    let resident_partition_bytes = resident_bytes_for_partition(tiles_per_partition, model);
    let active_partitions = active_partitions.max(1) as u64;
    let partition_bytes = resident_partition_bytes.checked_mul(active_partitions)?;
    budget
        .reserve_bytes
        .checked_add(model.shared_stage_bytes)?
        .checked_add(global_worker_workspace_bytes_for_model(model, budget))?
        .checked_add(partition_bytes)
}

fn resident_bytes_for_partition(tiles_per_partition: usize, model: &StageMemoryProfile) -> u64 {
    transient_partition_bytes(model).saturating_add(
        model
            .output_tile_bytes
            .saturating_mul(tiles_per_partition.max(1) as u64),
    )
}

fn transient_partition_bytes(model: &StageMemoryProfile) -> u64 {
    model.primary_tile_bytes.saturating_add(
        model
            .secondary_tile_bytes_per_input
            .saturating_mul(model.secondary_input_count as u64),
    )
}

fn global_worker_workspace_bytes_for_model(
    model: &StageMemoryProfile,
    budget: &ExecutionMemoryBudget,
) -> u64 {
    model
        .per_worker_workspace_bytes
        .saturating_mul(budget.worker_count.max(1) as u64)
}

fn better_candidate(
    candidate: &TraceLocalChunkPlan,
    current_best: Option<&TraceLocalChunkPlan>,
    mode: ChunkPlanningMode,
    preferred_partition_count: usize,
) -> bool {
    let Some(current_best) = current_best else {
        return true;
    };

    match mode {
        ChunkPlanningMode::Conservative => {
            conservative_key(candidate) < conservative_key(current_best)
        }
        ChunkPlanningMode::Auto => {
            auto_key(candidate, preferred_partition_count)
                < auto_key(current_best, preferred_partition_count)
        }
        ChunkPlanningMode::Throughput => {
            throughput_key(candidate, preferred_partition_count)
                < throughput_key(current_best, preferred_partition_count)
        }
    }
}

fn conservative_key(plan: &TraceLocalChunkPlan) -> impl Ord {
    (
        plan.max_active_partitions,
        plan.estimated_peak_bytes,
        partition_waves(plan),
        plan.partition_count,
        u64::MAX - plan.compatibility_target_bytes,
        usize::MAX - plan.tiles_per_partition,
    )
}

fn auto_key(plan: &TraceLocalChunkPlan, preferred_partition_count: usize) -> impl Ord {
    (
        partition_distance(plan, preferred_partition_count),
        partition_waves(plan),
        plan.estimated_peak_bytes,
        plan.partition_count,
        usize::MAX - plan.tiles_per_partition,
        plan.max_active_partitions,
        plan.compatibility_target_bytes,
    )
}

fn throughput_key(plan: &TraceLocalChunkPlan, preferred_partition_count: usize) -> impl Ord {
    (
        throughput_underpartition_penalty(plan, preferred_partition_count),
        partition_waves(plan),
        usize::MAX - plan.max_active_partitions,
        partition_distance(plan, preferred_partition_count),
        plan.estimated_peak_bytes,
        plan.partition_count,
        usize::MAX - plan.tiles_per_partition,
        plan.compatibility_target_bytes,
    )
}

fn partition_waves(plan: &TraceLocalChunkPlan) -> usize {
    plan.partition_count
        .div_ceil(plan.max_active_partitions.max(1))
}

fn partition_distance(plan: &TraceLocalChunkPlan, preferred_partition_count: usize) -> usize {
    plan.partition_count
        .abs_diff(preferred_partition_count.max(1))
}

fn throughput_underpartition_penalty(
    plan: &TraceLocalChunkPlan,
    preferred_partition_count: usize,
) -> usize {
    preferred_partition_count
        .max(1)
        .saturating_sub(plan.partition_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::Chunkability;

    #[test]
    fn compiler_prefers_higher_parallelism_for_throughput_when_budget_allows() {
        let model = StageMemoryProfile {
            chunkability: Chunkability::TileSpan,
            primary_tile_bytes: 1024,
            secondary_input_count: 0,
            secondary_tile_bytes_per_input: 0,
            output_tile_bytes: 1024,
            per_worker_workspace_bytes: 256,
            shared_stage_bytes: 0,
            reserve_hint_bytes: 0,
        };
        let budget = ExecutionMemoryBudget {
            usable_bytes: 64 * 1024,
            reserve_bytes: 0,
            worker_count: 4,
        };

        let conservative = compile_trace_local_chunk_plan(
            32,
            PartitionOrdering::StorageOrder,
            &[model.clone()],
            &budget,
            ChunkPlanningMode::Conservative,
            Some(4),
        )
        .expect("conservative plan");
        let throughput = compile_trace_local_chunk_plan(
            32,
            PartitionOrdering::StorageOrder,
            &[model],
            &budget,
            ChunkPlanningMode::Throughput,
            Some(4),
        )
        .expect("throughput plan");

        assert!(conservative.max_active_partitions <= throughput.max_active_partitions);
    }

    #[test]
    fn compiler_limits_tiles_per_partition_when_secondary_inputs_raise_resident_bytes() {
        let budget = ExecutionMemoryBudget {
            usable_bytes: 48 * 1024,
            reserve_bytes: 0,
            worker_count: 4,
        };
        let light = StageMemoryProfile {
            chunkability: Chunkability::TileSpan,
            primary_tile_bytes: 2048,
            secondary_input_count: 0,
            secondary_tile_bytes_per_input: 0,
            output_tile_bytes: 2048,
            per_worker_workspace_bytes: 128,
            shared_stage_bytes: 0,
            reserve_hint_bytes: 0,
        };
        let heavy = StageMemoryProfile {
            secondary_input_count: 1,
            secondary_tile_bytes_per_input: 2048,
            ..light.clone()
        };

        let light_plan = compile_trace_local_chunk_plan(
            16,
            PartitionOrdering::StorageOrder,
            &[light],
            &budget,
            ChunkPlanningMode::Auto,
            Some(4),
        )
        .expect("light plan");
        let heavy_plan = compile_trace_local_chunk_plan(
            16,
            PartitionOrdering::StorageOrder,
            &[heavy],
            &budget,
            ChunkPlanningMode::Auto,
            Some(4),
        )
        .expect("heavy plan");

        assert!(heavy_plan.tiles_per_partition <= light_plan.tiles_per_partition);
    }

    #[test]
    fn auto_mode_prefers_planner_baseline_partition_count_when_feasible() {
        let model = StageMemoryProfile {
            chunkability: Chunkability::TileSpan,
            primary_tile_bytes: 1024,
            secondary_input_count: 0,
            secondary_tile_bytes_per_input: 0,
            output_tile_bytes: 1024,
            per_worker_workspace_bytes: 256,
            shared_stage_bytes: 0,
            reserve_hint_bytes: 0,
        };
        let budget = ExecutionMemoryBudget {
            usable_bytes: 256 * 1024,
            reserve_bytes: 0,
            worker_count: 8,
        };

        let auto = compile_trace_local_chunk_plan(
            32,
            PartitionOrdering::StorageOrder,
            &[model],
            &budget,
            ChunkPlanningMode::Auto,
            Some(5),
        )
        .expect("auto plan");

        assert_eq!(auto.partition_count, 5);
        assert_eq!(auto.max_active_partitions, 5);
    }

    #[test]
    fn throughput_mode_respects_preferred_partition_floor() {
        let model = StageMemoryProfile {
            chunkability: Chunkability::TileSpan,
            primary_tile_bytes: 1024,
            secondary_input_count: 0,
            secondary_tile_bytes_per_input: 0,
            output_tile_bytes: 1024,
            per_worker_workspace_bytes: 256,
            shared_stage_bytes: 0,
            reserve_hint_bytes: 0,
        };
        let budget = ExecutionMemoryBudget {
            usable_bytes: 256 * 1024,
            reserve_bytes: 0,
            worker_count: 8,
        };

        let throughput = compile_trace_local_chunk_plan(
            32,
            PartitionOrdering::StorageOrder,
            &[model],
            &budget,
            ChunkPlanningMode::Throughput,
            Some(16),
        )
        .expect("throughput plan");

        assert!(throughput.partition_count >= 16);
        assert_eq!(partition_waves(&throughput), 2);
    }
}
