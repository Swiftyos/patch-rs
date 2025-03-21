use std::error::Error;
use std::fmt;

use crate::ast::{Line, Patch};

/// Error that can occur while applying a patch
#[derive(Debug)]
pub enum ApplyError {
    /// The line number in the patch is out of bounds for the input text
    LineOutOfBounds {
        /// The line number that was out of bounds
        line: u64,
        /// The total number of lines in the input text
        total_lines: usize,
    },
    /// The context lines in the patch don't match the input text
    ContextMismatch {
        /// The line number where the mismatch occurred
        line: u64,
        /// The expected context line from the patch
        expected: String,
        /// The actual line from the input text
        actual: String,
    },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::LineOutOfBounds { line, total_lines } => {
                write!(
                    f,
                    "Line {} is out of bounds (file has {} lines)",
                    line, total_lines
                )
            }
            ApplyError::ContextMismatch {
                line,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Context mismatch at line {}: expected '{}', got '{}'",
                    line, expected, actual
                )
            }
        }
    }
}

impl Error for ApplyError {}

/// Apply a patch to the given text content
///
/// # Arguments
///
/// * `patch` - The patch to apply
/// * `content` - The text content to apply the patch to
///
/// # Returns
///
/// The patched text content if successful, or an error if the patch cannot be applied
///
/// # Example
///
/// ```
/// use patch::{Patch, apply};
///
/// let content = "line 1\nline 2\nline 3\n";
/// let patch_text = "\
/// --- old.txt
/// +++ new.txt
/// @@ -1,3 +1,3 @@
///  line 1
/// -line 2
/// +new line 2
///  line 3
/// ";
///
/// let patch = Patch::from_single(patch_text).unwrap();
/// let result = apply(&patch, content).unwrap();
/// assert_eq!(result, "line 1\nnew line 2\nline 3\n");
/// ```
pub fn apply(patch: &Patch, content: &str) -> Result<String, ApplyError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut current_line = 0;

    for hunk in &patch.hunks {
        // Add unchanged lines before the hunk
        while current_line < (hunk.old_range.start - 1) as usize {
            if current_line >= lines.len() {
                return Err(ApplyError::LineOutOfBounds {
                    line: current_line as u64 + 1,
                    total_lines: lines.len(),
                });
            }
            result.push(lines[current_line].to_string());
            current_line += 1;
        }

        let mut hunk_old_line = current_line;
        for line in &hunk.lines {
            match line {
                Line::Context(text) => {
                    if hunk_old_line >= lines.len() {
                        return Err(ApplyError::LineOutOfBounds {
                            line: hunk_old_line as u64 + 1,
                            total_lines: lines.len(),
                        });
                    }
                    if lines[hunk_old_line] != *text {
                        return Err(ApplyError::ContextMismatch {
                            line: hunk_old_line as u64 + 1,
                            expected: text.to_string(),
                            actual: lines[hunk_old_line].to_string(),
                        });
                    }
                    result.push(text.to_string());
                    hunk_old_line += 1;
                }
                Line::Add(text) => {
                    result.push(text.to_string());
                }
                Line::Remove(text) => {
                    if hunk_old_line >= lines.len() {
                        return Err(ApplyError::LineOutOfBounds {
                            line: hunk_old_line as u64 + 1,
                            total_lines: lines.len(),
                        });
                    }
                    if lines[hunk_old_line] != *text {
                        return Err(ApplyError::ContextMismatch {
                            line: hunk_old_line as u64 + 1,
                            expected: text.to_string(),
                            actual: lines[hunk_old_line].to_string(),
                        });
                    }
                    hunk_old_line += 1;
                }
            }
        }
        current_line = hunk_old_line;
    }

    // Add any remaining lines after the last hunk
    while current_line < lines.len() {
        result.push(lines[current_line].to_string());
        current_line += 1;
    }

    // Handle the end newline based on the patch's end_newline flag
    let mut output = result.join("\n");
    if !output.is_empty() && patch.end_newline {
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Patch;

    #[test]
    fn test_apply_simple_patch() {
        let content = "line 1\nline 2\nline 3\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 line 1
-line 2
+new line 2
 line 3
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "line 1\nnew line 2\nline 3\n");
    }

    #[test]
    fn test_apply_patch_with_additions() {
        let content = "A\nB\nC\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,5 @@
 A
+X
 B
+Y
 C
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "A\nX\nB\nY\nC\n");
    }

    #[test]
    fn test_apply_patch_with_removals() {
        let content = "A\nB\nC\nD\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,4 +1,2 @@
 A
-B
-C
 D
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "A\nD\n");
    }


    #[test]
    fn test_apply_patch_line_out_of_bounds() {
        let content = "A\nB\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 A
 B
-C
+D
";
        let patch = Patch::from_single(patch_text).unwrap();
        let err = apply(&patch, content).unwrap_err();
        match err {
            ApplyError::LineOutOfBounds {
                line,
                total_lines,
            } => {
                assert_eq!(line, 3);
                assert_eq!(total_lines, 2);
            }
            _ => panic!("Expected LineOutOfBounds error"),
        }
    }

    #[test]
    fn test_apply_patch_context_mismatch() {
        let content = "A\nB\nC\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 A
-X
+Y
 C
";
        let patch = Patch::from_single(patch_text).unwrap();
        let err = apply(&patch, content).unwrap_err();
        match err {
            ApplyError::ContextMismatch {
                line,
                expected,
                actual,
            } => {
                assert_eq!(line, 2);
                assert_eq!(expected, "X");
                assert_eq!(actual, "B");
            }
            _ => panic!("Expected ContextMismatch error"),
        }
    }
} 