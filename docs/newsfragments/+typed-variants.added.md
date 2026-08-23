Every method has a typed request and response form. Nineteen did not.

`Request::parse` and `decode_response` answered "no typed form; send it untyped" for
`append`, `vote`, `fold`, `check` and the rest, so a client wanting typed access to
them could not have it. The two enums had also drifted from the method list by exactly
the amount nobody was checking.

Four result types carry the replies: `ReportResult` for the reads that produce prose,
`CheckResult` where the error and warning counts travel beside the text, `DigestResult`
and `WaitResult` where there is structure worth having. Reports share one type on
purpose — a type per report would be a contract per report to keep in step with the
text, and the text is the part anyone reads.

A test drives the round-trip from the capability list, so a method reaching the wire
without a typed form fails there rather than being discovered by whoever wanted it.
