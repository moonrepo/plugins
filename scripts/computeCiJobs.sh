#!/usr/bin/env bash


raw=$(moon query tasks --affected --upstream deep --downstream deep)

echo "Tasks:"
echo "$raw"
echo ""

data=$(moon query tasks --affected --upstream deep --downstream deep --json)
taskCount=$(echo "$data" | jq '.tasks | length')
taskPerJob=15
jobTotal=1

if [[ $taskCount == 0 ]]; then
    jobTotal=0
elif [[ $taskCount -gt 15 ]]; then
    ((jobTotal = (taskCount + taskPerJob - 1) / taskPerJob))
fi

jobsArray="["

if [[ $jobTotal -gt 0 ]]; then
    for i in $(seq 1 $jobTotal);
    do
        jobIndex=$((i - 1))
        jobsArray+="$jobIndex"

        if [[ $i -ne $jobTotal ]]; then
            jobsArray+=","
        fi
    done
fi

jobsArray+="]"

echo "Task count: $taskCount"
echo "Job total: $jobTotal"
echo "Job array: $jobsArray"

echo "job-total=$jobTotal" >> "$GITHUB_OUTPUT"
echo "jobs-array=$jobsArray" >> "$GITHUB_OUTPUT"
