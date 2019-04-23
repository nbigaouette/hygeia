#!/bin/bash

# tarpaulin_image_version="latest"
tarpaulin_image_version="@sha256:e94bf79f5d60fd021383cdc011182066999a395223976b071fc9f8b75d10a35c"

####################################################################################
# Tarpaulin
# https://hub.docker.com/r/xd009642/tarpaulin

echo "Backup stable build directory..."
cmd="mv target/debug target/debug.stable"
echo ${cmd}
eval ${cmd}

echo "Moving tarpaulin build directory back in place (if exists)..."
cmd="[[ -d target/debug.tarpaulin ]] && mv target/debug.tarpaulin target/debug"
echo ${cmd}
eval ${cmd}

cmd="time docker run --security-opt seccomp=unconfined -v '${PWD}:/volume' xd009642/tarpaulin${tarpaulin_image_version} sh -c 'cargo tarpaulin --out Xml'"
echo ${cmd}
eval ${cmd}


echo "Backup tarpaulin build directory..."
cmd="mv target/debug target/debug.tarpaulin"
echo ${cmd}
eval ${cmd}


echo "Moving stable build directory back in place..."
cmd="mv target/debug.stable target/debug"
echo ${cmd}
eval ${cmd}

####################################################################################

# Generate HTML report using pycobertura

if [[ -d venv_pycobertura ]]; then
    cmd="source venv_pycobertura/bin/activate"
    echo ${cmd}
    eval ${cmd}
else
    cmd="python -m venv venv_pycobertura"
    echo ${cmd}
    eval ${cmd}

    cmd="source venv_pycobertura/bin/activate"
    echo ${cmd}
    eval ${cmd}

    cmd="pip install pycobertura==0.10.5"
    echo ${cmd}
    eval ${cmd}
fi

cmd="pycobertura show --format html --output code_coverage/tarpaulin.html cobertura.xml"
echo ${cmd}
eval ${cmd}
