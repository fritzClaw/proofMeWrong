/**
 * @name Output outside the nosecrets choke points
 * @description Agent code may write output only through nosecrets::audit
 *              (the log) and nosecrets::deliver (the user), and may read input
 *              only through nosecrets::schema::next_command. Independent second
 *              layer next to the clippy ban list (INTENT.md §3.2).
 * @kind problem
 * @problem.severity error
 * @security-severity 7.5
 * @precision high
 * @id proofmewrong/output-outside-choke-points
 * @tags security
 *       external/cwe/cwe-532
 */

import rust

bindingset[p]
predicate bannedPath(string p) {
  exists(string prefix |
    prefix =
      [
        "std::io::", "std::fs::", "std::net::", "std::process::", "std::env::", "std::os::",
        "<std::io::", "<std::fs::", "<std::net::", "<std::process::", "<std::env::", "<std::os::"
      ]
  |
    p.matches(prefix + "%")
  )
}

predicate inAgentCode(AstNode n) { n.getLocation().getFile().getRelativePath().matches("app/%") }

from AstNode n, string what
where
  inAgentCode(n) and
  (
    exists(Call c, string p |
      n = c and p = c.getStaticTarget().getCanonicalPath() and bannedPath(p) and what = p
    )
    or
    exists(MacroCall mc, string name |
      n = mc and
      name = mc.getPath().getText() and
      name = ["print", "println", "eprint", "eprintln", "dbg", "write", "writeln"] and
      what = name + "!"
    )
  )
select n, "Use of " + what + " bypasses the nosecrets::audit / nosecrets::deliver choke points."
