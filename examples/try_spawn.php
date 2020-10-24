<?php

$i = 100;
while ($i > 0) {
    --$i;
    echo "I'm proccess still alive: at:" . time() . PHP_EOL;
    sleep(1);
}
