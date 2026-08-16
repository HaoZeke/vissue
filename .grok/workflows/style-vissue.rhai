let meta = #{
    name: "style-vissue",
    description: "Implementer plus two reviewers per remaining style vissue",
    phases: [
        #{ title: "Implement", detail: "one implementer per crate or issue, isolated worktree" },
        #{ title: "Review", detail: "two independent reviewers per implementation" },
    ],
};

let impl_schema = #{
    "type": "object",
    "required": ["issue", "crate_name", "summary", "files", "done"],
    "properties": #{
        "issue": #{ "type": "string" },
        "crate_name": #{ "type": "string" },
        "summary": #{ "type": "string" },
        "files": #{ "type": "array", "items": #{ "type": "string" } },
        "done": #{ "type": "boolean" },
        "blockers": #{ "type": "string" },
    },
};

let review_schema = #{
    "type": "object",
    "required": ["issue", "crate_name", "verdict", "blocking", "notes"],
    "properties": #{
        "issue": #{ "type": "string" },
        "crate_name": #{ "type": "string" },
        "verdict": #{ "type": "string" },
        "blocking": #{ "type": "boolean" },
        "notes": #{ "type": "string" },
    },
};

let wave = if args == () { () } else { args.wave };
if wave == () { pause("verification", "Pass args.wave as 3 or 4."); }

if wave != "3" && wave != "4" {
    pause("verification", "args.wave must be 3 (docs) or 4 (edition/anyhow). Wave 2 is sequential on main.");
}

let repo = "the current vissue checkout";
let issues_cli = "vissue show";

let impl_jobs = [];

if wave == "3" {
    let crates = ["vissue-core", "vissue-control", "vissue-serve", "vissue-tui"];
    for c in crates {
        let p = "";
        p += "You are implementing vault issue vissue-b15k (and the overlapping rustdoc on vissue-z5vw) ";
        p += "for ONE crate only: " + c + " in " + repo + ".\n\n";
        p += "Read the issue bodies with `" + issues_cli + " vissue-b15k` and `" + issues_cli + " vissue-z5vw`.\n";
        p += "Read crates/" + c + "/src thoroughly with grep and read_file before writing.\n\n";
        p += "Goal: every public item in crates/" + c + " gets rustdoc. Public fns that return Result get an # Errors section. ";
        p += "Public fns that panic on a documented contract get # Panics. Match the existing crate voice: short, present tense, no temporal comments, ASCII punctuation only.\n\n";
        p += "Do NOT edit Cargo.toml workspace lints. Do NOT enable missing_docs or clippy missing_errors_doc yourself. ";
        p += "Do NOT edit other crates. Do NOT cargo build, cargo test, cargo clippy, or cargo doc on this laptop; if you must compile, ssh rg.terra after rsync. ";
        p += "Do NOT open GitHub PRs or issues. Do NOT commit unless the crate rustdoc pass is actually finished.\n\n";
        p += "C-FAILURE: # Errors names the conditions, not the type. Display text of errors is lowercase, no trailing period.\n";
        p += "Do not invent APIs. Do not rewrite behavior. Comments explain WHY only when the constraint is non-obvious.\n";
        p += "If a public type cannot be documented without reading a sibling module, read that module first.\n\n";
        p += "Return {issue, crate_name, summary, files, done, blockers}. done=true only if every public item in this crate has rustdoc.";
        impl_jobs.push(#{
            prompt: p,
            label: "impl-b15k-" + c,
            capability_mode: "all",
            isolation_worktree: true,
            output_schema: impl_schema,
        });
    }
}

if wave == "4" {
    let p_wlpz = "";
    p_wlpz += "Implement vault issue vissue-wlpz in " + repo + ".\n";
    p_wlpz += "Read the body with `" + issues_cli + " vissue-wlpz`.\n";
    p_wlpz += "Libraries (vissue-core, vissue-control, vissue-serve, vissue-tui) must stop returning anyhow::Result on public and crate-visible fns. ";
    p_wlpz += "Use the existing vissue_core::error::Error (and crate-local Error types). Keep anyhow in vissue-cli, vissue-mcp, vissue-hud.\n";
    p_wlpz += "Do not add thiserror until the typed surface is decided; prefer mapping through existing Error variants.\n";
    p_wlpz += "C-GOOD-ERR: Error + Display + Debug + Send + Sync. Display is lowercase, no trailing period.\n";
    p_wlpz += "NEVER cargo build/test locally. Use ssh rg.terra. Do not open GitHub PRs. Isolation worktree only.\n";
    p_wlpz += "Return {issue: vissue-wlpz, crate_name: libraries, summary, files, done, blockers}.";
    impl_jobs.push(#{
        prompt: p_wlpz,
        label: "impl-wlpz",
        capability_mode: "all",
        isolation_worktree: true,
        output_schema: impl_schema,
    });

    let p_os3b = "";
    p_os3b += "Implement vault issue vissue-os3b in " + repo + ".\n";
    p_os3b += "Read the body with `" + issues_cli + " vissue-os3b`.\n";
    p_os3b += "Step 1 only unless step 1 is already clean: set workspace.lints.rust.rust_2024_compatibility = warn and fix warnings.\n";
    p_os3b += "Do NOT flip edition to 2024 and do NOT set resolver = 3 until step 1 is clean on rustc 1.89 via ssh rg.terra.\n";
    p_os3b += "Do not set build.warnings = deny. NEVER cargo build locally. Do not open GitHub PRs.\n";
    p_os3b += "Return {issue: vissue-os3b, crate_name: workspace, summary, files, done, blockers}.";
    impl_jobs.push(#{
        prompt: p_os3b,
        label: "impl-os3b",
        capability_mode: "all",
        isolation_worktree: true,
        output_schema: impl_schema,
    });
}

phase("Implement");
log("Launching " + impl_jobs.len().to_string() + " implementers for wave " + wave);
let impls = parallel(impl_jobs);

let reviews = [];
let i = 0;
while i < impls.len() {
    let r = impls[i];
    if r != () && r.success && r.output != () {
        reviews.push(r.output);
    }
    i += 1;
}

if reviews.len() == 0 {
    complete(#{ summary: "No implementer produced a usable result.", wave: wave });
}

phase("Review");
let vjobs = [];
let ri = 0;
while ri < reviews.len() {
    let item = reviews[ri];
    let issue = item.issue;
    let crate_name = item.crate_name;
    let summary = item.summary;
    let files = json_encode(item.files);

    let n = 0;
    while n < 2 {
        let rp = "";
        rp += "You are reviewer " + (n + 1).to_string() + " of 2, adversarially reviewing rust style work.\n";
        rp += "Issue: " + issue + ". Crate or scope: " + crate_name + ".\n";
        rp += "Implementer summary: " + summary + "\n";
        rp += "Files they claim: " + files + "\n\n";
        rp += "Read the shipped files in " + repo + " AND any worktree the implementer used. ";
        rp += "Use grep and read_file. Do not trust the summary. Do not edit source.\n\n";
        rp += "Fail closed: blocking=true if public items still lack rustdoc (wave 3), if anyhow::Result remains on library APIs (wlpz), ";
        rp += "if edition flipped without rust_2024_compatibility first (os3b), if pedantic/restriction/nursery were enabled as groups, ";
        rp += "if comments narrate the change, or if they cargo-built on the laptop.\n";
        rp += "verdict is one of: accept, accept-with-nits, reject.\n";
        rp += "Return {issue, crate_name, verdict, blocking, notes} with notes citing file:line.";
        vjobs.push(#{
            prompt: rp,
            label: "review-" + crate_name + "-" + (n + 1).to_string(),
            capability_mode: "read-only",
            output_schema: review_schema,
        });
        n += 1;
    }
    ri += 1;
}

let verdicts = parallel(vjobs);
let accepted = 0;
let rejected = 0;
let vi = 0;
while vi < verdicts.len() {
    let v = verdicts[vi];
    if v != () && v.success && v.output != () {
        if v.output.blocking == true || v.output.verdict == "reject" {
            rejected += 1;
        } else {
            accepted += 1;
        }
    } else {
        rejected += 1;
    }
    vi += 1;
}

let report = "wave " + wave + ": " + accepted.to_string() + " accepting reviews, " + rejected.to_string() + " rejecting or unusable";
log(report);
complete(#{
    summary: report,
    wave: wave,
    implementers: reviews.len(),
    accepting_reviews: accepted,
    rejecting_reviews: rejected,
});
