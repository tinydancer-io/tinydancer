#!/bin/bash
echo "\033[0;32mstarting rpc connection(might error if already connected)..."
./rpc
echo "\033[0;32mbuilding cli..."
sh path

tinydancer

read cont
if [ "$cont" != "y" ]; then
    exit 1
fi

tput setaf 2; echo "testing config..."
tinydancer config get
tinydancer config set --cluster Localnet --log-path /tmp/client.logPath
# tinydancer config get

echo "Continue? (y/n)"
read cont
if [ "$cont" != "y" ]; then
    exit 1
fi

echo "\033[0;32mtesting slot..."
slot=$(tinydancer slot)
slot_number=$(echo $slot | cut -d':' -f 2 | xargs | sed 's/\x1B\[[0-9;]\{1,\}[A-Za-z]//g')
sleep 1
tinydancer verify --slot "$slot_number"

echo "Continue? (y/n)"
read cont
if [ "$cont" != "y" ]; then
    exit 1
fi

echo "\033[0;32mtesting daemon..."
# should handle gracefully
tinydancer start
sleep 1
tinydancer start "/tmp"
PID=$! 
kill -INT $PID # kill if still running

echo "Continue? (y/n)"
read cont
if [ "$cont" != "y" ]; then
    exit 1
fi


echo "\033[0;32mtests complete"