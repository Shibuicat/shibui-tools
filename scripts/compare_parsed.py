import glob
import json
import os
import sys

OLD_DIR, NEW_DIR = sys.argv[1], sys.argv[2]
RENAMES = {"pronounces": "pronunciations"}


def as_new_shape(value):
    if isinstance(value, list):
        return [as_new_shape(item) for item in value]
    if not isinstance(value, dict):
        return value
    converted = {}
    for key, item in value.items():
        if key == "examples" and isinstance(item, list):
            converted[key] = [{"text": e} if isinstance(e, str) else as_new_shape(e) for e in item]
        else:
            converted[RENAMES.get(key, key)] = as_new_shape(item)
    return converted


def main():
    pages = sorted(glob.glob(os.path.join(OLD_DIR, "*", "*.json")))
    different = []
    for old_path in pages:
        new_path = old_path.replace(OLD_DIR, NEW_DIR, 1)
        old, new = open(old_path).read(), open(new_path).read()
        if old.startswith("ERROR") or new.startswith("ERROR"):
            same = old == new
        else:
            same = as_new_shape(json.loads(old)) == json.loads(new)
        if not same:
            different.append(old_path)
    print(f"pages {len(pages)}, different {len(different)}")
    for path in different:
        print("DIFF", path)
    sys.exit(1 if different else 0)


main()
