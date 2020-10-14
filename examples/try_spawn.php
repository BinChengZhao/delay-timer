<?php

sleep(3);
file_put_contents('./test.txt', "aaaaaaaaaaaaaaaaaaa");
$i = 100;
while($i>0){
    sleep(1);
    $i--;
    echo "I'm proccess still alive: at:".time().PHP_EOL;
}