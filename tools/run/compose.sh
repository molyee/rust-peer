#!/bin/bash

set -e

# getting external ip address and docker ip address
case "$(uname -s)" in
   Darwin)
     export DOCKER_IP=host.docker.internal
     export HOST_IP=host.docker.internal
     ;;

   Linux)
     export DOCKER_IP=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}' | grep -Po "[0-9\.]+")
     export HOST_IP=$(ip route get 8.8.8.8 | grep -Po "(?<=src )[0-9\.]+")
     ;;
esac

# running parity and swarm containers
docker-compose -f parity.yml up -d
docker-compose -f swarm.yml up -d

echo 'Parity and Swarm containers are started.'

# waiting that API of parity start working
# todo get rid of all `sleep`
sleep 20

echo 'Check if Parity node is synced.'

# waiting that parity synced
# todo add this logic to CLI
syncing=false
while [ $syncing == false ]
do
    result=$(curl -s -X POST --data '{"jsonrpc":"2.0","method":"eth_syncing","params":[],"id":1}' -H "Content-Type: application/json" http://localhost:8545)
    if [[ $result == *"result\":false"* ]]; then
        syncing=true
    else
        echo 'Parity node is not synced. New attempt after 20 seconds.'
        sleep 20
    fi
done

# starting node container
docker-compose -f node.yml up -d --force-recreate

echo 'Node container is started.'

# check all variables exists
echo "HOST_IP="$HOST_IP
echo "OWNER_ADDRESS="$OWNER_ADDRESS
echo "CONTRACT_ADDRESS="$CONTRACT_ADDRESS
echo "PRIVATE_KEY="$PRIVATE_KEY

# get tendermint key from node logs
# todo get this from `status` API by CLI
while [ -z "$TENDERMINT_KEY" ]
do
    sleep 3
    TENDERMINT_KEY=$(docker logs node1 2>&1 | grep PubKey | sed 's/.*value\":\"\([^ ]*\).*/\1/' | sed 's/\"},//g')
done

echo "TENDERMINT_KEY="$TENDERMINT_KEY

# check if node is already registered
./fluence register $HOST_IP $TENDERMINT_KEY $OWNER_ADDRESS $CONTRACT_ADDRESS -s $PRIVATE_KEY --wait_syncing --base64
