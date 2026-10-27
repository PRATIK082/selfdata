# AUTOSAR YAML templates

These YAML files describe an automotive system in a reviewable, diff-friendly
form. They are the **source of truth** for requirements, design, implementation,
integration and test artifacts. A (draft) converter turns them into ARXML:

```text
python tools/yaml2arxml/yaml2arxml.py templates/autosar/system.yaml -o out.arxml
```

Planned coverage (iterative):

- requirements traceability
- system / ECU topology
- software components and ports
- port interfaces and data elements
- runnables and internal behavior
- network (CAN/LIN/Ethernet) frames, PDUs and signals
- vehicle variants and ECU configurations
- integration and system test scenarios