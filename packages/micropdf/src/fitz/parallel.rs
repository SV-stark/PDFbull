//! Parallel processing utilities using Rayon
//!
//! This module provides utilities for parallel processing of PDF operations,
//! enabling significant performance improvements on multi-core systems.

use rayon::prelude::*;
use std::path::Path;

use crate::fitz::buffer::Buffer;
use crate::fitz::error::Result;
use crate::fitz::pixmap::Pixmap;
use crate::fitz::stream::Stream;

/// Process multiple buffers in parallel using a custom function.
///
/// # Example
/// ```ignore
/// use micropdf::fitz::parallel::process_buffers;
/// use micropdf::fitz::buffer::Buffer;
///
/// let buffers = vec![Buffer::from_slice(b"data1"), Buffer::from_slice(b"data2")];
/// let results: Vec<usize> = process_buffers(&buffers, |b| b.len());
/// ```
pub fn process_buffers<F, R>(buffers: &[Buffer], f: F) -> Vec<R>
where
    F: Fn(&Buffer) -> R + Sync + Send,
    R: Send,
{
    buffers.par_iter().map(f).collect()
}

/// Process multiple buffers with results in parallel.
pub fn process_buffers_result<F, R>(buffers: &[Buffer], f: F) -> Vec<Result<R>>
where
    F: Fn(&Buffer) -> Result<R> + Sync + Send,
    R: Send,
{
    buffers.par_iter().map(f).collect()
}

/// Transform buffer data in parallel chunks.
///
/// Splits the buffer into chunks and applies the transformation function
/// to each chunk in parallel, then reassembles the results.
pub fn parallel_transform<F>(buffer: &Buffer, chunk_size: usize, f: F) -> Buffer
where
    F: Fn(&[u8]) -> Vec<u8> + Sync + Send,
{
    let data = buffer.to_vec();
    let chunks: Vec<Vec<u8>> = data.par_chunks(chunk_size).map(f).collect();

    let total_len: usize = chunks.iter().map(|c| c.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    for chunk in chunks {
        result.extend(chunk);
    }
    Buffer::from_data(result)
}

/// Read multiple files in parallel.
///
/// # Example
/// ```ignore
/// use micropdf::fitz::parallel::read_files;
///
/// let paths = vec!["file1.pdf", "file2.pdf", "file3.pdf"];
/// let results = read_files(&paths);
/// ```
pub fn read_files<P: AsRef<Path> + Sync>(paths: &[P]) -> Vec<Result<Buffer>> {
    paths
        .par_iter()
        .map(|path| {
            let mut stream = Stream::open_file(path)?;
            stream.read_all(0)
        })
        .collect()
}

/// Process multiple pixmaps in parallel.
pub fn process_pixmaps<F, R>(pixmaps: &[Pixmap], f: F) -> Vec<R>
where
    F: Fn(&Pixmap) -> R + Sync + Send,
    R: Send,
{
    pixmaps.par_iter().map(f).collect()
}

/// Apply a pixel transformation to multiple pixmaps in parallel.
pub fn transform_pixmaps<F>(pixmaps: Vec<Pixmap>, f: F) -> Vec<Pixmap>
where
    F: Fn(Pixmap) -> Pixmap + Sync + Send,
{
    pixmaps.into_par_iter().map(f).collect()
}

/// Batch process items with a parallel iterator.
pub fn batch_process<T, F, R>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send,
    F: Fn(T) -> R + Sync + Send,
    R: Send,
{
    items.into_par_iter().map(f).collect()
}

/// Batch process items with results.
pub fn batch_process_result<T, F, R>(items: Vec<T>, f: F) -> Vec<Result<R>>
where
    T: Send,
    F: Fn(T) -> Result<R> + Sync + Send,
    R: Send,
{
    items.into_par_iter().map(f).collect()
}

/// Filter items in parallel.
pub fn parallel_filter<T, F>(items: Vec<T>, predicate: F) -> Vec<T>
where
    T: Send,
    F: Fn(&T) -> bool + Sync + Send,
{
    items.into_par_iter().filter(predicate).collect()
}

/// Find items in parallel that match a predicate.
pub fn parallel_find<T, F>(items: &[T], predicate: F) -> Option<&T>
where
    T: Sync,
    F: Fn(&T) -> bool + Sync + Send,
{
    items.par_iter().find_any(|item| predicate(item))
}

/// Count items matching a predicate in parallel.
pub fn parallel_count<T, F>(items: &[T], predicate: F) -> usize
where
    T: Sync,
    F: Fn(&T) -> bool + Sync + Send,
{
    items.par_iter().filter(|item| predicate(*item)).count()
}

/// Sum values extracted from items in parallel.
pub fn parallel_sum<T, F>(items: &[T], f: F) -> i64
where
    T: Sync,
    F: Fn(&T) -> i64 + Sync + Send,
{
    items.par_iter().map(f).sum()
}

/// Get the number of available parallel threads.
pub fn num_threads() -> usize {
    rayon::current_num_threads()
}

/// Execute work with a specific thread pool size.
pub fn with_thread_count<F, R>(num_threads: usize, f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("Failed to create thread pool");
    pool.install(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_buffers() {
        let buffers = vec![
            Buffer::from_slice(&[1, 2, 3]),
            Buffer::from_slice(&[4, 5]),
            Buffer::from_slice(&[6, 7, 8, 9]),
        ];

        let lengths: Vec<usize> = process_buffers(&buffers, |b| b.len());
        assert_eq!(lengths, vec![3, 2, 4]);
    }

    #[test]
    fn test_parallel_transform() {
        let buffer = Buffer::from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let result = parallel_transform(&buffer, 2, |chunk| chunk.iter().map(|b| b * 2).collect());

        assert_eq!(result.to_vec(), vec![2, 4, 6, 8, 10, 12, 14, 16]);
    }

    #[test]
    fn test_batch_process() {
        let items = vec![1, 2, 3, 4, 5];
        let results: Vec<i32> = batch_process(items, |x| x * x);
        assert_eq!(results, vec![1, 4, 9, 16, 25]);
    }

    #[test]
    fn test_parallel_filter() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let evens = parallel_filter(items, |x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_parallel_count() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let count = parallel_count(&items, |x| *x > 5);
        assert_eq!(count, 5);
    }

    #[test]
    fn test_parallel_sum() {
        let items = vec![1i64, 2, 3, 4, 5];
        let sum = parallel_sum(&items, |x| *x);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_num_threads() {
        let threads = num_threads();
        assert!(threads > 0);
    }

    #[test]
    fn test_with_thread_count() {
        let result = with_thread_count(2, || {
            assert!(rayon::current_num_threads() <= 2);
            42
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_process_buffers_result() {
        let buffers = vec![Buffer::from_slice(&[1, 2, 3]), Buffer::from_slice(&[4, 5])];

        let results: Vec<Result<usize>> = process_buffers_result(&buffers, |b| Ok(b.len()));

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap(), &3);
    }

    #[test]
    fn test_process_pixmaps() {
        let pixmaps = vec![
            Pixmap::new(None, 10, 10, true).unwrap(),
            Pixmap::new(None, 20, 20, true).unwrap(),
        ];

        let areas: Vec<usize> = process_pixmaps(&pixmaps, |p| (p.width() * p.height()) as usize);
        assert_eq!(areas, vec![100, 400]);
    }

    #[test]
    fn test_transform_pixmaps() {
        let pixmaps = vec![
            Pixmap::new(None, 10, 10, true).unwrap(),
            Pixmap::new(None, 20, 20, true).unwrap(),
        ];

        let transformed = transform_pixmaps(pixmaps, |p| {
            // Just return the pixmap as-is
            p
        });

        assert_eq!(transformed.len(), 2);
        assert_eq!(transformed[0].width(), 10);
        assert_eq!(transformed[1].width(), 20);
    }

    #[test]
    fn test_batch_process_result() {
        let items = vec![1, 2, 3, 4, 5];
        let results: Vec<Result<i32>> = batch_process_result(items, |x| Ok(x * 2));

        assert_eq!(results.len(), 5);
        assert_eq!(results[0].as_ref().unwrap(), &2);
        assert_eq!(results[4].as_ref().unwrap(), &10);
    }

    #[test]
    fn test_parallel_find() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let found = parallel_find(&items, |x| *x == 7);

        assert!(found.is_some());
        assert_eq!(*found.unwrap(), 7);
    }

    #[test]
    fn test_parallel_find_none() {
        let items = vec![1, 2, 3, 4, 5];
        let found = parallel_find(&items, |x| *x > 10);

        assert!(found.is_none());
    }

    #[test]
    fn test_parallel_transform_empty() {
        let buffer = Buffer::new(0);
        let result = parallel_transform(&buffer, 2, |chunk| chunk.to_vec());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parallel_filter_empty() {
        let items: Vec<i32> = vec![];
        let filtered = parallel_filter(items, |x| *x > 0);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_parallel_count_zero() {
        let items = vec![1, 2, 3];
        let count = parallel_count(&items, |x| *x > 10);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_parallel_sum_empty() {
        let items: Vec<i64> = vec![];
        let sum = parallel_sum(&items, |x| *x);
        assert_eq!(sum, 0);
    }
}
