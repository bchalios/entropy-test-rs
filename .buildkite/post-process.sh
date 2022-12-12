#! /bin/bash


echo "instance_type kernel rng_type size #requests mean stddev max min p50 p90 p99 p999" > results.txt
for instance in "m5d" "m6i" "m6a" "m6gd"
do
  for kernel in "4_14" "5_10"
  do
    for rng in "os-rng" "thread-rng"
    do
      for size in 64 512 1024
      do
        artifact="results_${instance}.metal_${kernel/_/.}_${size}_${rng}.txt"
        buildkite-agent artifact download ${artifact} . --step "${instance}_${kernel}_${size}_${rng}"
        echo -n "${instance} ${kernel} ${rng} ${size} " >> results.txt
        cat ${artifact} >> results.txt
        echo "" >> results.txt
      done
    done
  done
done

buildkite-agent artifact upload results.txt
