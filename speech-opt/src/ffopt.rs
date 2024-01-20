use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use regex::Regex;

/// 获取音频的静音间隔，「duration」指的是静音判断间隔，实际调用如下命令：
/// ```bash
/// ffmpeg -i 文件名 -af silencedetect=d=时间间隔 -f null -
/// ```
pub fn detect_silence<P>(audio: P, duration: Duration) -> Result<Vec<(Duration, Duration)>>
where
    P: AsRef<OsStr>,
{
    let ff = Command::new("ffmpeg")
        .arg("-i")
        .arg(audio.as_ref())
        .arg("-af")
        .arg(format!("silencedetect=d={}", duration.as_secs_f64()))
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()?;

    let result = String::from_utf8_lossy(&ff.stderr);

    let regex_end = Regex::new(
        r"\[silencedetect.*?silence_end: (?P<silence_end>.*?) \| silence_duration: (?P<silence_duration>.*)",
    ).unwrap();

    let mut silence_range = Vec::new();

    for line in result.lines() {
        if let Some(cap) = regex_end.captures(line) {
            let silence_end = cap.name("silence_end").unwrap().as_str().parse().unwrap();
            let silence_end = Duration::from_secs_f64(silence_end);

            let silence_duration = cap
                .name("silence_duration")
                .unwrap()
                .as_str()
                .parse()
                .unwrap();

            let silence_duration = Duration::from_secs_f64(silence_duration);
            let silence_start = silence_end - silence_duration;

            silence_range.push((silence_start, silence_end));
        }
    }

    Ok(silence_range)
}

/// 裁剪音频，实际调用如下命令：
/// ```bash
/// ffmpeg -ss 起始时间点 -i 输入文件 -c copy -t 时间间隔 输出文件
/// ```
pub fn cut_audio<P>(audio: P, start: Duration, duration: Duration, output: P) -> Result<()>
where
    P: AsRef<OsStr>,
{
    let _ff = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(format!("{}", start.as_secs_f64()))
        .arg("-i")
        .arg(audio)
        .args(["-c", "copy"])
        .arg("-t")
        .arg(format!("{}", duration.as_secs_f64()))
        .arg(output)
        .output()?;

    Ok(())
}

/// 按照时间点合成音频，实际调用如下命令：
/// ```bash
/// ffmpeg -i 音频1 -i 音频2 以此类推
///   -filter_complex
///   "[1]adelay=184000|184000[b];
///    [2]adelay=360000|360000[c];
///    [3]adelay=962000|962000[d];
///    [0][b][c][d]amix=4"
/// 输出文件
/// ```
pub fn join_audios(info_list: &[(Duration, PathBuf)], output: &PathBuf) -> Result<()> {
    let mut ff_command = Command::new("ffmpeg");
    ff_command.arg("-y");
    let mut filter_complex = Vec::new();
    let mut index_vec = Vec::new();

    for i in 0..info_list.len() {
        let (start, audio) = &info_list[i];
        let start_spot = start.as_millis();

        ff_command.arg("-i");
        ff_command.arg(audio);

        filter_complex.push(format!(
            "[{}]adelay={}|{}[{}]",
            i,
            start_spot,
            start_spot,
            format!("a{}", i)
        ));

        index_vec.push(format!("[a{}]", i));
    }

    let amix = format!("{}amix={}", index_vec.join(""), info_list.len());
    filter_complex.push(amix);
    let filter_complex = filter_complex.join(";");

    ff_command.arg("-filter_complex");
    ff_command.arg(filter_complex);

    ff_command.arg(output);

    let _ff = ff_command.output()?;

    Ok(())
}

fn get_duration_from_output(output: &str) -> Option<String> {
    let regex = regex::Regex::new(r"Duration: ([0-9:\.]*), start:.*bitrate:").unwrap();
    let caps = regex
        .captures(&output)
        .map(|x| x.get(1).unwrap().as_str())
        .map(|x| x.to_string());
    caps
}

/// 获取音频的长度
/// ```bash
/// ffmpeg -i 音频名称
/// ```
pub fn get_audio_duration<P>(audio: P) -> Result<Duration>
where
    P: AsRef<OsStr>,
{
    let output = Command::new("ffmpeg").arg("-i").arg(audio).output()?;
    let output = String::from_utf8(output.stderr)?;

    let duration = get_duration_from_output(&output)
        .ok_or_else(|| anyhow::anyhow!("failed to get duration"))?;

    let duration = match duration.split(":").collect::<Vec<&str>>().as_slice() {
        [seconds] => Some(seconds.parse()?),
        [minutes, seconds] => Some((minutes.parse::<u64>()? * 60) as f64 + seconds.parse::<f64>()?),
        [hours, minutes, seconds] => Some(
            (hours.parse::<u64>()? * 3600) as f64
                + (minutes.parse::<u64>()? * 60) as f64
                + seconds.parse::<f64>()?,
        ),
        [days, hours, minutes, seconds] => Some(
            (days.parse::<u64>()? * 3600) as f64
                + (hours.parse::<u64>()? * 3600) as f64
                + (minutes.parse::<u64>()? * 60) as f64
                + seconds.parse::<f64>()?,
        ),
        _ => None,
    };

    duration
        .map(|x| Duration::from_secs_f64(x))
        .ok_or_else(|| anyhow::anyhow!("failed to get duration"))
}

pub fn get_audio_pieces<P>(audio: P, duration: Duration) -> Result<Vec<(Duration, Duration)>>
where
    P: AsRef<OsStr>,
{
    let silence_pairs = detect_silence(&audio, duration)?;
    let mut timeline = Vec::new();
    timeline.push(Duration::from_millis(0));
    for (start, end) in silence_pairs {
        timeline.push(start);
        timeline.push(end);
    }

    let end_time = get_audio_duration(&audio)?;
    timeline.push(end_time);

    let mut result = Vec::new();
    let audio_length = timeline.len() / 2;
    for i in 0..audio_length {
        result.push((timeline[i * 2], timeline[i * 2 + 1]));
    }

    Ok(result)
}

pub fn cut_audio2(
    audio: &PathBuf,
    audio_pieces: &[(Duration, Duration)],
    temp_dir: &PathBuf,
) -> Result<Vec<(Duration, Duration, PathBuf)>> {
    let file_name = audio.file_name().unwrap();
    let mut result = Vec::new();
    for i in 0..audio_pieces.len() {
        let (start, end) = audio_pieces[i];
        let dur = end - start;
        let temp_file_name = format!("{}#{}.mp3", file_name.to_string_lossy(), i);
        let temp_output = temp_dir.join(temp_file_name);
        cut_audio(audio, start, dur, &temp_output)?;
        result.push((start, dur, temp_output));
    }
    Ok(result)
}

pub fn parse_timestamp(timestamp: &str) -> Result<Duration> {
    let token: Vec<&str> = timestamp.split(&[':', ','][..]).collect();
    let millis = match token.as_slice() {
        [hours, minutes, seconds, millis] => {
            let seconds = (hours.parse::<u64>()? * 3600)
                + (minutes.parse::<u64>()? * 60)
                + seconds.parse::<u64>()?;
            let millis = seconds * 1000 + millis.parse::<u64>()?;
            Ok(millis)
        }
        _ => Err(anyhow::anyhow!("Failed to parse timestamp")),
    };

    millis.map(|x| Duration::from_millis(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_silence() {
        let actual = vec![(3.38717, 5.49354), (8.47175, 10.584)];
        let result =
            detect_silence(PathBuf::from("sample.mp3"), Duration::from_secs_f64(0.2)).unwrap();

        for i in 0..2 {
            let (actual_start, actual_end) = actual[i];
            let (result_start, result_end) = result[i];

            assert!((actual_start - result_start.as_secs_f64()) < 0.02);
            assert!((actual_end - result_end.as_secs_f64()) < 0.01);
        }
    }

    #[test]
    fn test_get_duration_from_output() {
        let output = "  Duration: 00:00:12.86, start: 0.046042, bitrate: 32 kb/s";
        let duration = get_duration_from_output(output).unwrap();
        assert_eq!("00:00:12.86", duration)
    }

    #[test]
    fn test_get_audio_duration() {
        let duration = get_audio_duration("./sample.mp3").unwrap();
        assert!((12.86 - duration.as_secs_f64()).abs() < 0.02)
    }

    #[test]
    fn test_parse_timestamp() {
        let timestamp = "00:00:4,960";
        let dur = parse_timestamp(timestamp).unwrap();
        assert_eq!(dur.as_millis(), 4 * 1000 + 960)
    }
}
