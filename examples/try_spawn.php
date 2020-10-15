<?php
file_put_contents('./test1.txt', "u dropd i alive test1");
sleep(2);
$i = 100;
while($i>0){
    $i--;
    echo "I'm proccess still alive: at:".time().PHP_EOL;
    file_put_contents('./test.txt', "u dropd i alive.".time());
    sleep(1);
}