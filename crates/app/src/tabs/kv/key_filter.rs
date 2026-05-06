use std::ops::Range;

use crate::tabs::common::matches_query;
use crate::tabs::types::{CachedKvKeyRows, KvBucketState, SearchCacheKey};

pub(crate) fn invalidate(state: &mut KvBucketState) {
    state.invalidate_filtered_key_cache();
}

pub(crate) fn filtered_row_count(state: &mut KvBucketState) -> usize {
    if !state.search.is_active() {
        return state.keys.len();
    }
    filtered_key_indices(state).len()
}

pub(crate) fn visible_key_indices(
    state: &mut KvBucketState,
    row_range: Range<usize>,
) -> Vec<usize> {
    if !state.search.is_active() {
        let start = row_range.start.min(state.keys.len());
        let end = row_range.end.min(state.keys.len());
        return (start..end).collect();
    }

    let rows = filtered_key_indices(state);
    let start = row_range.start.min(rows.len());
    let end = row_range.end.min(rows.len());
    rows[start..end].to_vec()
}

fn filtered_key_indices(state: &mut KvBucketState) -> &[usize] {
    let cache_key = SearchCacheKey::from_state(&state.search);
    let selected_key = state.selected_key.clone();
    let needs_refresh = match &state.cached_filtered_keys {
        Some(cached) => {
            cached.generation != state.search_generation
                || cached.cache_key != cache_key
                || cached.selected_key != selected_key
        }
        None => true,
    };

    if needs_refresh {
        let query = cache_key.query.as_str();
        let rows = state
            .keys
            .iter()
            .enumerate()
            .filter_map(|(idx, key)| {
                let key_matches = cache_key.primary && matches_query(key, query);
                let fetched_value_matches = cache_key.secondary
                    && state
                        .fetched_values
                        .get(key.as_str())
                        .is_some_and(|value| matches_query(value, query));
                let history_matches = cache_key.secondary
                    && selected_key.as_deref() == Some(key.as_str())
                    && state
                        .history
                        .iter()
                        .any(|item| matches_query(&searchable_kv_history_item(item), query));
                (key_matches || fetched_value_matches || history_matches).then_some(idx)
            })
            .collect();

        state.cached_filtered_keys = Some(CachedKvKeyRows {
            generation: state.search_generation,
            cache_key,
            selected_key,
            rows,
        });
    }

    &state
        .cached_filtered_keys
        .as_ref()
        .expect("filtered key cache is built")
        .rows
}

fn searchable_kv_history_item(item: &nats_backend::KvHistoryItem) -> String {
    String::from_utf8_lossy(&item.value).into_owned()
}

#[cfg(test)]
mod tests {
    use crate::tabs::types::KvBucketState;

    use super::*;

    #[test]
    fn inactive_search_counts_loaded_keys_without_filter_cache() {
        let mut state = KvBucketState {
            keys: (0..300_000).map(|idx| format!("key.{idx}")).collect(),
            ..Default::default()
        };

        assert_eq!(filtered_row_count(&mut state), 300_000);
        assert_eq!(
            visible_key_indices(&mut state, 20..25),
            vec![20, 21, 22, 23, 24]
        );
        assert!(state.cached_filtered_keys.is_none());
    }

    #[test]
    fn active_key_search_caches_matching_indices() {
        let mut state = KvBucketState {
            keys: vec![
                "orders.1".to_string(),
                "users.alice".to_string(),
                "orders.2".to_string(),
            ],
            ..Default::default()
        };
        state.search.query = "orders".to_string();
        state.search.primary = true;
        state.search.secondary = false;

        assert_eq!(visible_key_indices(&mut state, 0..2), vec![0, 2]);
        assert_eq!(filtered_row_count(&mut state), 2);
        assert!(state.cached_filtered_keys.is_some());
    }

    #[test]
    fn fetched_value_generation_refreshes_cached_matches() {
        let mut state = KvBucketState {
            keys: vec!["users.alice".to_string(), "users.bob".to_string()],
            ..Default::default()
        };
        state.search.query = "42".to_string();
        state.search.primary = false;
        state.search.secondary = true;

        assert_eq!(filtered_row_count(&mut state), 0);

        state
            .fetched_values
            .insert("users.bob".to_string(), "balance: 42".to_string());
        state.search_generation = state.search_generation.wrapping_add(1);

        assert_eq!(visible_key_indices(&mut state, 0..1), vec![1]);
        assert_eq!(filtered_row_count(&mut state), 1);
    }

    #[test]
    fn invalidate_drops_cached_filtered_indices() {
        let mut state = KvBucketState {
            keys: vec!["orders.1".to_string()],
            ..Default::default()
        };
        state.search.query = "orders".to_string();

        assert_eq!(filtered_row_count(&mut state), 1);
        assert!(state.cached_filtered_keys.is_some());

        invalidate(&mut state);

        assert!(state.cached_filtered_keys.is_none());
    }
}
