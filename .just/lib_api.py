import json
import sys


def fmt_type(t):
    if "primitive" in t:
        return t["primitive"]
    if "resolved_path" in t:
        p = t["resolved_path"]
        name = p["path"].split("::")[-1]
        args = p.get("args")
        return name + fmt_args(args) if args else name
    if "borrowed_ref" in t:
        r = t["borrowed_ref"]
        lt = f"{r['lifetime']} " if r.get("lifetime") else ""
        mut = "mut " if r["is_mutable"] else ""
        return f"&{lt}{mut}{fmt_type(r['type'])}"
    if "tuple" in t:
        items = t["tuple"]
        return "()" if not items else "(" + ", ".join(fmt_type(i) for i in items) + ")"
    if "impl_trait" in t:
        bounds = [b for b in (fmt_bound(b) for b in t["impl_trait"]) if b]
        return "impl " + " + ".join(bounds)
    if "dyn_trait" in t:
        d = t["dyn_trait"]
        traits = [tr["trait"]["path"].split("::")[-1] for tr in d["traits"]]
        return "dyn " + " + ".join(traits)
    if "generic" in t:
        return t["generic"]
    if "raw_pointer" in t:
        r = t["raw_pointer"]
        return f"*{'mut' if r['is_mutable'] else 'const'} {fmt_type(r['type'])}"
    if "slice" in t:
        return f"[{fmt_type(t['slice'])}]"
    if "array" in t:
        return f"[{fmt_type(t['array']['type'])}; {t['array']['len']}]"
    if "qualified_path" in t:
        q = t["qualified_path"]
        return f"<{fmt_type(q['self_type'])} as {q['trait']['path'].split('::')[-1]}>::{q['name']}"
    if "infer" in t:
        return "_"
    return "?"


def fmt_args(args):
    if "angle_bracketed" in args:
        parts = []
        for arg in args["angle_bracketed"]["args"]:
            if "type" in arg:
                parts.append(fmt_type(arg["type"]))
            elif "lifetime" in arg:
                parts.append(arg["lifetime"])
            elif "const" in arg:
                parts.append(str(arg["const"]))
        return "<" + ", ".join(parts) + ">" if parts else ""
    if "parenthesized" in args:
        p = args["parenthesized"]
        inputs = ", ".join(fmt_type(i) for i in p["inputs"])
        output = p.get("output")
        return f"({inputs}) -> {fmt_type(output)}" if output else f"({inputs})"
    return ""


def fmt_bound(b):
    if "trait_bound" in b:
        tb = b["trait_bound"]["trait"]
        name = tb["path"].split("::")[-1]
        args = tb.get("args")
        return name + fmt_args(args) if args else name
    if "lifetime" in b:
        return b["lifetime"]
    return None


def fmt_generics(generics, strip_lifetimes=False):
    parts = []
    for p in generics.get("params", []):
        name, kind = p["name"], p.get("kind", {})
        if "lifetime" in kind:
            if strip_lifetimes:
                continue
            parts.append(name)
        elif "type" in kind:
            bounds = [b for b in (fmt_bound(b) for b in kind["type"].get("bounds", [])) if b]
            if name.startswith("impl "):
                parts.append(name)
            else:
                parts.append(f"{name}: {' + '.join(bounds)}" if bounds else name)
        elif "const" in kind:
            parts.append(f"const {name}: {fmt_type(kind['const']['type'])}")
    return "<" + ", ".join(parts) + ">" if parts else ""


def extract_async_output(t):
    """Unwrap Pin<Box<dyn Future<Output=T> + Send>> from async_trait desugaring."""
    try:
        if "Pin" not in t["resolved_path"]["path"]:
            return None
        box_type = t["resolved_path"]["args"]["angle_bracketed"]["args"][0]["type"]["resolved_path"]
        if "Box" not in box_type["path"]:
            return None
        dyn = box_type["args"]["angle_bracketed"]["args"][0]["type"]["dyn_trait"]
        for trait in dyn["traits"]:
            if "Future" in trait["trait"]["path"]:
                for c in trait["trait"]["args"]["angle_bracketed"]["constraints"]:
                    if c["name"] == "Output":
                        return c["binding"]["equality"]["type"]
    except (KeyError, IndexError, TypeError):
        pass
    return None


def fmt_param(name, ptype):
    if name == "self":
        if "borrowed_ref" in ptype:
            r = ptype["borrowed_ref"]
            if r["type"].get("generic") == "Self":
                return "&mut self" if r["is_mutable"] else "&self"
        if ptype.get("generic") == "Self":
            return "self"
    return f"{name}: {fmt_type(ptype)}"


def fmt_sig(name, fn):
    sig, header, generics = fn["sig"], fn.get("header", {}), fn.get("generics", {})
    output = sig.get("output")
    async_output = extract_async_output(output) if output else None
    is_async = header.get("is_async") or async_output is not None
    prefix = "pub "
    if is_async:
        prefix += "async "
    if header.get("is_unsafe"):
        prefix += "unsafe "
    gen = fmt_generics(generics, strip_lifetimes=async_output is not None)
    params = ", ".join(fmt_param(pn, pt) for pn, pt in sig["inputs"])
    if async_output is not None:
        ret = f" -> {fmt_type(async_output)}"
    elif output:
        ret = f" -> {fmt_type(output)}"
    else:
        ret = ""
    return f"{prefix}fn {name}{gen}({params}){ret}"


pkg = sys.argv[1]
with open(sys.argv[2]) as f:
    doc = json.load(f)

index = doc["index"]
results = []

# Free public functions
for item in index.values():
    if (item.get("crate_id") == 0
            and item.get("visibility") == "public"
            and item.get("inner", {}).get("function") is not None):
        results.append((item["name"], fmt_sig(item["name"], item["inner"]["function"])))

# Methods on public traits defined in this crate
for item in index.values():
    if (item.get("crate_id") == 0
            and item.get("visibility") == "public"
            and item.get("inner", {}).get("trait") is not None):
        trait_name = item["name"]
        for child_id in item["inner"]["trait"].get("items", []):
            child = index.get(str(child_id))
            if child and child.get("inner", {}).get("function") is not None:
                fn_name = f"{trait_name}::{child['name']}"
                results.append((fn_name, fmt_sig(fn_name, child["inner"]["function"])))

for name, sig in sorted(results):
    print(f"{pkg}::{sig}")
