const RAW_ENVIRONMENTS: &[&str] = &[
    "verbatim", "verbatim*", "Verbatim", "Verbatim*", "minted", "minted*", "lstlisting",
    "lstlisting*", "comment", "filecontents", "filecontents*", "tabular", "tabular*",
    "longtable", "tabu", "array", "tikzpicture", "tikzpicture*", "pgfpicture",
    "pgfpicture*", "forest", "forest*", "algorithm", "algorithm*", "algorithmic",
    "algorithmic*", "thebibliography", "thebibliography*", "bibliography", "references",
];

const MATH_ENVIRONMENTS: &[&str] = &[
    "equation", "equation*", "align", "align*", "alignat", "alignat*", "flalign", "flalign*",
    "gather", "gather*", "multline", "multline*", "eqnarray", "eqnarray*", "math",
    "displaymath", "split", "cases", "matrix", "pmatrix", "bmatrix", "vmatrix", "Vmatrix",
];

const TEX_BEGIN_PREFIX: &str = "\\begin{";
const TEX_END_PREFIX: &str = "\\end{";

pub(super) fn begin_environment_name(line: &str) -> Option<&str> {
    parse_environment_name(line.trim_start(), TEX_BEGIN_PREFIX)
}

pub(super) fn end_environment_name(line: &str) -> Option<&str> {
    parse_environment_name(line.trim_start(), TEX_END_PREFIX)
}

pub(super) fn is_raw_environment_name(name: &str) -> bool {
    RAW_ENVIRONMENTS.contains(&name)
}

pub(super) fn is_math_environment_name(name: &str) -> bool {
    MATH_ENVIRONMENTS.contains(&name)
}

fn parse_environment_name<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if !line.starts_with(prefix) {
        return None;
    }
    let start = prefix.len();
    let end = line[start..].find('}')? + start;
    let name = &line[start..end];
    (!name.is_empty()).then_some(name)
}
