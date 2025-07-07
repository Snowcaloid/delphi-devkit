import { promises as fs } from 'fs';
import { dirname, join, basename, isAbsolute, resolve } from 'path';

/**
 * Given a .dproj file path, returns the expected .exe output path.
 * - Reads <DCC_ExeOutput> from the .dproj (regex, not XML parsing)
 * - Uses the .dpr filename (without extension) as the exe name
 * - Output path is relative to the .dproj file
 */
export async function getExpectedExePathFromDproj(dprojPath: string, dprPath?: string): Promise<string | null> {
  try {
    const dprojContent = await fs.readFile(dprojPath, 'utf8');
    // Find <DCC_ExeOutput>...</DCC_ExeOutput>
    const exeOutputMatch = dprojContent.match(/<DCC_ExeOutput>(.*?)<\/DCC_ExeOutput>/i);
    let exeOutput = exeOutputMatch ? exeOutputMatch[1].trim() : '';
    const dprojDir = dirname(dprojPath);
    if (!exeOutput) {
      exeOutput = dprojDir;
    } else if (!isAbsolute(exeOutput)) {
      exeOutput = resolve(dprojDir, exeOutput);
    }
    // Determine exe name: use .dpr filename if provided, else .dproj filename
    let exeName = '';
    if (dprPath) {
      exeName = basename(dprPath, '.dpr') + '.exe';
    } else {
      exeName = basename(dprojPath, '.dproj') + '.exe';
    }
    // Join and normalize
    const exePath = join(exeOutput, exeName);
    return exePath;
  } catch (err) {
    console.error('[Delphi][ExePath] Error:', err);
    return null;
  }
}
