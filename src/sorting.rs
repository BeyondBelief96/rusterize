//! Sorting algorithm implementations for reference.
//!
//! These are educational implementations. For production use,
//! prefer the standard library's `sort_by` method.

use crate::rasterizer::Triangle;

/// Bubble sort triangles by avg_depth in descending order (furthest first).
///
/// Time complexity: O(n²)
/// Space complexity: O(1)
#[allow(dead_code)]
pub fn bubble_sort_by_depth_descending(triangles: &mut [Triangle]) {
    let len = triangles.len();
    for i in 0..len {
        for j in 0..len - i - 1 {
            if triangles[j].avg_depth < triangles[j + 1].avg_depth {
                triangles.swap(j, j + 1);
            }
        }
    }
}

/// Merge sort triangles by avg_depth in descending order (furthest first).
///
/// Time complexity: O(n log n)
/// Space complexity: O(n)
#[allow(dead_code)]
pub fn merge_sort_by_depth_descending(triangles: &mut Vec<Triangle>) {
    let len = triangles.len();
    if len <= 1 {
        return;
    }

    let mid = len / 2;
    let mut left = triangles[..mid].to_vec();
    let mut right = triangles[mid..].to_vec();

    merge_sort_by_depth_descending(&mut left);
    merge_sort_by_depth_descending(&mut right);

    *triangles = merge_descending(left, right);
}

/// Merge two sorted vectors into one, maintaining descending order by avg_depth.
fn merge_descending(left: Vec<Triangle>, right: Vec<Triangle>) -> Vec<Triangle> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let mut left_iter = left.into_iter().peekable();
    let mut right_iter = right.into_iter().peekable();

    while left_iter.peek().is_some() && right_iter.peek().is_some() {
        // Descending: take the larger depth first
        if left_iter.peek().unwrap().avg_depth >= right_iter.peek().unwrap().avg_depth {
            result.push(left_iter.next().unwrap());
        } else {
            result.push(right_iter.next().unwrap());
        }
    }

    result.extend(left_iter);
    result.extend(right_iter);
    result
}
