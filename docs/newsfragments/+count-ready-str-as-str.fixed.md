`vissue count --ready` compiled on a newer rustc by calling `str::as_str`,
which 1.97.1 still marks unstable. The filter already has `&str`.
