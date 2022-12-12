#!/usr/bin/env python3
# Copyright 2022 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0

"""Generate Buildkite pipelines dynamically"""

import json

INSTANCES = [
    "m5d.metal",
    "m6i.metal",
    "m6a.metal",
    "m6gd.metal",
]

KERNELS = ["4.14", "5.10"]


def group(
    group_name,
    pipeline_id,
    depends_on,
    instance,
    kernel,
    agent_tags=None,
    priority=0,
    timeout=30,
):
    """
    Generate a group step with specified parameters, for each instance+kernel
    combination

    https://buildkite.com/docs/pipelines/group-step
    """
    if agent_tags is None:
        agent_tags = []
    # Use the 1st character of the group name (should be an emoji)
    label1 = group_name[0]
    steps = []
    agents = [
        f"type={instance}",
        f"kv={kernel}",
    ]
    agents.extend(agent_tags)
    for rng in ["os-rng", "thread-rng"]:
        for size in [64, 512, 1024]:
            stats_file = f"results_{instance}_{kernel}_{size}_{rng}.txt"
            step = {
                "command": [
                    f"buildkite-agent artifact download entropy-test-{instance} --step \"build_{instance.replace('.metal', '')}\" .",
                    f"mv entropy-test-{instance} entropy-test",
                    "chmod u+x entropy-test",
                    f"docker run --rm -ti -v $(pwd):/test -w /test rust:1.65-buster ./perf.sh {rng} {size} {stats_file}",
                ],
                "label": f"{label1} {instance} kv={kernel} request-size={size} rng:{rng}",
                "key": f"{instance.replace('.metal', '')}_{kernel.replace('.', '_')}_{size}_{rng}",
                "priority": priority,
                "timeout": timeout,
                "agents": agents,
                "artifact_paths": [f"{stats_file}"]
            }
            steps.append(step)

    return {
        "group": group_name,
        "id": pipeline_id,
        "depends_on": depends_on,
        "steps": steps,
    }

def build_group():
    group_steps = []
    for instance in INSTANCES:
        agents = [f"type={instance}", "ag=4"]
        step = {
            "command": [
                'docker run --rm -ti -v $(pwd):/test -w /test rust:1.65-buster /bin/bash -c " apt update && apt install libclang-dev -y && cargo build --release"',
                f"cp target/release/entropy-test entropy-test-{instance}",
            ],
            "artifact_paths": [f"entropy-test-{instance}"],
            "label": f"Build test on {instance}",
            "timeout": 30,
            "agents": agents,
            "id": f"build_{instance.replace('.metal', '')}"
        }
        group_steps.append(step)

    return {"group": "Build test", "id": "build", "steps": group_steps}


steps = [build_group()]

for instance in INSTANCES:
    for kernel in KERNELS:
            g = group(
                f"Test on {instance} with {kernel}",
                f"run_{instance.replace('.metal', '')}_{kernel.replace('.', '_')}",
                f"build_{instance.replace('.metal', '')}",
                instance,
                kernel,
                agent_tags=["ag=1"],
            )
            steps.append(g)

steps.append("wait")
steps.append({
    "command": ".buildkite/post-process.sh",
    "label": "Post process",
})

pipeline = {
    "agents": {"queue": "default"},
    "steps": steps,
}

print(json.dumps(pipeline, indent=4, sort_keys=True, ensure_ascii=False))
