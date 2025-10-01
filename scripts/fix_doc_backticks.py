#!/usr/bin/env python3
"""
Add backticks to function names in doc comments
"""
import re
import glob

def fix_doc_line(line):
    """Fix a single doc comment line by adding backticks around function names."""
    if not line.lstrip().startswith('///'):
        return line

    # Pattern 1: Function names with ()
    pattern1 = r'(\s)([a-zA-Z_][a-zA-Z0-9_]*\(\))(\s)'
    line = re.sub(pattern1, r'\1`\2`\3', line)

    # Pattern 2: snake_case identifiers with underscores (like fish_add_path)
    # Match if followed by whitespace OR punctuation
    pattern2 = r'(\s)([a-z_][a-z0-9_]*_[a-z0-9_]+)(\s|[.,;)\]])'
    line = re.sub(pattern2, r'\1`\2`\3', line)

    # Pattern 3: PascalCase type names (like PathBuf)
    pattern3 = r'(\s)([A-Z][a-zA-Z0-9]*[A-Z][a-zA-Z0-9]*)(\s)'
    line = re.sub(pattern3, r'\1`\2`\3', line)

    return line

# Find all Rust files
rust_files = glob.glob('src/**/*.rs', recursive=True)

total_changes = 0
for file_path in rust_files:
    with open(file_path, 'r') as f:
        lines = f.readlines()

    new_lines = []
    file_changes = 0
    for line in lines:
        new_line = fix_doc_line(line)
        if new_line != line:
            file_changes += 1
        new_lines.append(new_line)

    if file_changes > 0:
        with open(file_path, 'w') as f:
            f.writelines(new_lines)
        print(f"✓ {file_path}: {file_changes} lines updated")
        total_changes += file_changes

print(f"\nTotal: {total_changes} doc comments updated")
