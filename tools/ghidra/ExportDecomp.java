// Bulk decompiler for reVI: one JSON object per function, keyed by entry RVA.
// Usage: -postScript ExportDecomp.java <out.jsonl> [rva-list.txt]
// With a list, only those entry RVAs are decompiled; otherwise everything.
//@category reVI

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.PrintWriter;
import java.util.HashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;

public class ExportDecomp extends GhidraScript {

    private static String json(String s) {
        StringBuilder b = new StringBuilder(s.length() + 16);
        for (char c : s.toCharArray()) {
            switch (c) {
                case '"' -> b.append("\\\"");
                case '\\' -> b.append("\\\\");
                case '\n' -> b.append("\\n");
                case '\r' -> b.append("\\r");
                case '\t' -> b.append("\\t");
                default -> {
                    if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
                    else b.append(c);
                }
            }
        }
        return b.toString();
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            printerr("usage: ExportDecomp.java <out.jsonl> [rva-list.txt]");
            return;
        }
        Set<Long> only = new HashSet<>();
        if (args.length > 1) {
            try (BufferedReader r = new BufferedReader(new FileReader(args[1]))) {
                String line;
                while ((line = r.readLine()) != null) {
                    line = line.trim();
                    if (!line.isEmpty()) only.add(Long.decode(line));
                }
            }
        }

        DecompInterface dec = new DecompInterface();
        dec.toggleCCode(true);
        dec.toggleSyntaxTree(true);
        dec.openProgram(currentProgram);

        FunctionManager fm = currentProgram.getFunctionManager();
        long done = 0, failed = 0;
        try (PrintWriter out = new PrintWriter(new java.io.BufferedWriter(new java.io.FileWriter(args[0])))) {
            for (Function f : fm.getFunctions(true)) {
                if (monitor.isCancelled()) break;
                long rva = f.getEntryPoint().getOffset();
                if (!only.isEmpty() && !only.contains(rva)) continue;
                DecompileResults res = dec.decompileFunction(f, 90, monitor);
                String code = null;
                if (res != null && res.decompileCompleted() && res.getDecompiledFunction() != null) {
                    code = res.getDecompiledFunction().getC();
                } else {
                    failed++;
                }
                out.println("{\"rva\":" + rva + ",\"name\":\"" + json(f.getName()) + "\",\"c\":\""
                        + json(code == null ? "" : code) + "\"}");
                done++;
                if (done % 200 == 0) {
                    out.flush();
                    println("exported " + done + ", failed " + failed);
                }
            }
        }
        println("done: exported " + done + ", failed " + failed);
    }
}
