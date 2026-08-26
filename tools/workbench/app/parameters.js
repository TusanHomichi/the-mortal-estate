/* The vocabulary's parameter list, rendered as typed inputs and read back.
 *
 * Every field on screen exists because the vocabulary declared it: its name, its
 * shape, and the sentence under it are the compiler's words, and a verb that
 * grows a parameter grows a field here without anyone editing this file. That is
 * the whole reason this is generated rather than written out — a hand-built form
 * per verb is a copy of the vocabulary, and a copy of the vocabulary is wrong
 * the first time the vocabulary changes.
 *
 * **The cells are prefilled from the selection.** A `[{x, y}]` parameter arrives
 * holding the cells of the packet the operation derives from, and a `{x, y}`
 * parameter holds the first of them. The owner pointed at those cells; a tool
 * that made them type the coordinates back would have replaced a measurement
 * with a guess, which is the exact substitution the packet exists to prevent.
 *
 * **Nothing here judges a value.** An empty nullable field is null because the
 * shape says `| null`, and a JSON field that does not parse refuses locally
 * rather than being sent as a string the server would have to interpret. Beyond
 * that, whether a class exists, whether a landmark is real and whether a cell is
 * on the map are the compiler's answers, and this file does not anticipate any
 * of them.
 */

import { TRUTH } from "./state.js";

const CELL_LIST = "[{x, y}]";
const CELL = "{x, y}";

export function option(value, label) {
  const element = document.createElement("option");
  element.value = value;
  element.textContent = label;
  return element;
}

/* Build the fields for one verb. Truth verbs are typed by the vocabulary; asset
 * verbs declare field names and not shapes, so each is entered as JSON and
 * parsed by the server's own contract parser. The adapter block is a field of
 * the record rather than a parameter, and is kept separate here for the same
 * reason it is kept separate there.
 */
export function renderParameters(holder, operationClass, spec, cells) {
  holder.innerHTML = "";
  if (operationClass === TRUTH) {
    for (const parameter of spec.parameters) holder.append(truthField(parameter, cells));
    return;
  }
  for (const name of spec.required) {
    if (name === "adapter") continue;
    holder.append(jsonField(name, `required · ${name} · entered as JSON`));
  }
  for (const name of spec.optional) {
    if (name === "adapter") continue;
    holder.append(jsonField(name, `optional · ${name} · entered as JSON`));
  }
  holder.append(
    jsonField("adapter", 'the adapter block · {"adapter": …, "parameters": {…}}', true)
  );
}

function truthField(parameter, cells) {
  /* A parameter with a closed set gets a picker that cannot produce a value the
   * compiler would refuse. The set is carried on the parameter itself, so this
   * reads no prose and guesses nothing: a verb whose vocabulary gains or loses a
   * choice changes this form without anyone editing this file.
   */
  if (parameter.choices) return classField(parameter, parameter.choices);
  if (parameter.shape === CELL_LIST) return field(parameter, JSON.stringify(cells), true);
  if (parameter.shape.startsWith(CELL)) {
    return field(parameter, cells.length > 0 ? JSON.stringify(cells[0]) : "", true);
  }
  return field(parameter, "", !parameter.shape.startsWith("string"));
}

function field(parameter, value, json) {
  const wrapper = label(parameter.name, `${parameter.shape} · ${parameter.summary}`);
  const input = document.createElement("input");
  input.type = "text";
  input.value = value;
  input.dataset.name = parameter.name;
  input.dataset.shape = parameter.shape;
  input.dataset.json = json ? "yes" : "no";
  input.className = "parameter";
  wrapper.append(input);
  return wrapper;
}

function classField(parameter, classes) {
  const wrapper = label(parameter.name, `${parameter.shape} · ${parameter.summary}`);
  const select = document.createElement("select");
  select.dataset.name = parameter.name;
  select.dataset.shape = parameter.shape;
  select.dataset.json = "no";
  select.className = "parameter";
  if (parameter.shape.includes("null")) select.append(option("", "(null)"));
  for (const name of classes) select.append(option(name, name));
  wrapper.append(select);
  return wrapper;
}

function jsonField(name, summary, adapter) {
  const wrapper = label(name, summary);
  const input = document.createElement("input");
  input.type = "text";
  input.dataset.name = name;
  input.dataset.json = "yes";
  input.dataset.adapter = adapter ? "yes" : "no";
  input.className = "parameter";
  wrapper.append(input);
  return wrapper;
}

function label(name, summary) {
  const wrapper = document.createElement("label");
  wrapper.className = "parameter-row";
  const head = document.createElement("span");
  head.className = "parameter-name";
  head.textContent = name;
  const note = document.createElement("span");
  note.className = "parameter-note";
  note.textContent = summary;
  wrapper.append(head, note);
  return wrapper;
}

/* Read the fields back: the parameters object the record carries, and the
 * adapter block that rides beside it. A JSON field that does not parse throws
 * here, naming the field and the parser's own complaint.
 */
export function readParameters(holder) {
  const values = {};
  let adapter = null;
  for (const input of holder.querySelectorAll(".parameter")) {
    const raw = input.value.trim();
    let value = raw;
    if (input.dataset.json === "yes") {
      if (raw === "") {
        value = null;
      } else {
        try {
          value = JSON.parse(raw);
        } catch (error) {
          throw new Error(`${input.dataset.name}: ${raw} is not JSON (${error.message})`);
        }
      }
    } else if (raw === "" && (input.dataset.shape || "").includes("null")) {
      value = null;
    }
    if (input.dataset.adapter === "yes") {
      adapter = value;
      continue;
    }
    values[input.dataset.name] = value;
  }
  return { values, adapter };
}
