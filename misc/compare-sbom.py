import json
import sys
from dataclasses import dataclass


if len(sys.argv) != 3:
    print("pass two files in parameter")
    sys.exit(1)

first_file = sys.argv[1]
second_file = sys.argv[2]

def load_components(path: str):
    res = {}
    with open(path, 'r') as file:
        first_data = json.load(file)

        first_components = first_data.get("components")
        for component in first_components:
            name = component.get("name")
            type = component.get("type")
            version = component.get("version")
            purl = component.get("purl")
            if type == "library" and purl is not None and "maven" in purl:
                res[name] = version

    return res

first_file_components = load_components(first_file)
second_file_components = load_components(second_file)


## Let's find components common two both with different versions
for name, version in first_file_components.items():
    if name in second_file_components and version != second_file_components[name]:
        print(f"Component {name} has different versions: {version} and {second_file_components[name]}")


## Let's find components in the first file absent from the second
for name, version in first_file_components.items():
    if name not in second_file_components:
        print(f"Component {name} is in the first file but not in the second")

## Let's find components in the second file absent from the first
for name, version in second_file_components.items():
    if name not in first_file_components:
        print(f"Component {name} is in the second file but not in the first")