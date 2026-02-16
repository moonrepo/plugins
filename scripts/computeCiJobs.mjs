// @ts-check
import fs from "node:fs";
import { exec } from "./utils.mjs";

const data = await exec(
  "moon",
  [
    "query",
    "tasks",
    "--affected",
    "--upstream",
    "deep",
    "--downstream",
    "deep",
  ],
  {
    stdio: "pipe",
  }
);
const { tasks } = JSON.parse(data.out);

console.log("Tasks:");
console.log(tasks);
console.log();

const taskCount = tasks.length;
const taskPerJob = 10;
const jobs = [];
let jobTotal = 1;

if (taskCount == 0) {
  jobTotal = 0;
} else if (taskCount > 10) {
  jobTotal = Math.ceil((taskCount + taskPerJob - 1) / taskPerJob);
}

for (let i = 0; i < jobTotal; i += 1) {
  jobs.push(i);
}

console.log("Task count:", taskCount);
console.log("Job total:", jobTotal);
console.log("Jobs:", jobs);

if (process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(
    process.env.GITHUB_OUTPUT,
    `job-total=${jobTotal}\njobs-array=${JSON.stringify(jobs)}\n`
  );
}
