export const PROBLEMMATCHER_REGEX =
  // Pretty format: [WARN][W1029] C:\path\file.pas (line 412): message
  // Capture groups: [1]=WARN, [2]=W1029, [3]=filepath, [4]=line, [5]=message
  /^\[(\w+)\]\[(\w+)\] (.*?) \(line (\d+)\): (.*)$/;
