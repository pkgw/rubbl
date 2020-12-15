//! Hack for Paul La Plante: synthesize a 352-antenna dataset from a smaller
//! one, so that we can see how our algorithms scale.

use clap::{App, Arg};
use failure::{format_err, Error, ResultExt};
use rubbl_miriad::mask::{MaskDecoder, MaskEncoder};
use rubbl_miriad::visdata::{
    decode_baseline, encode_baseline, Decoder, Encoder, UvVariableReference,
};
use rubbl_miriad::{DataSet, ReadStream, Type, WriteStream};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::process;
use std::time::Instant;

const NANTS: usize = 352;

fn main() {
    let matches = App::new("hera352")
        .version("0.1.0")
        .about("Make a fake 352-antenna HERA UV dataset")
        .arg(
            Arg::with_name("INPATH")
                .help("The path to the input dataset directory")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPATH")
                .help("The path to the (preexistin!) output dataset directory")
                .required(true)
                .index(2),
        )
        .get_matches();

    let in_path = matches.value_of_os("INPATH").unwrap();
    let out_path = matches.value_of_os("OUTPATH").unwrap();

    process::exit(
        match UvInflator::process(in_path.as_ref(), out_path.as_ref()) {
            Ok(code) => code,

            Err(e) => {
                println!(
                    "fatal error while processing {} => {}",
                    in_path.to_string_lossy(),
                    out_path.to_string_lossy()
                );
                for cause in e.iter_chain() {
                    println!("  caused by: {}", cause);
                }
                1
            }
        },
    );
}

struct UvInflator {
    pb: pbr::ProgressBar<io::Stdout>,

    in_uv: Decoder,
    in_flags: MaskDecoder<ReadStream>,
    in_n: usize,

    out_ds: DataSet,
    out_uv: Encoder,
    out_flags: MaskEncoder<WriteStream>,
    out_n: usize,

    /// hera_to_mir[hera_index] = miriad_index
    hera_to_mir: Vec<usize>,
    mir_to_hera: Vec<usize>,

    /// out_ant_to_in[output_hera_index] = input_hera_index.
    out_ant_to_in: Vec<usize>,

    /// the last seen time in the dataset
    time: f64,
    lst: f64,
    ra: f64,

    /// key is the input miriad antenna indices
    records: HashMap<(usize, usize), Record>,

    /// variables we use a lot
    time_var: UvVariableReference,
    lst_var: UvVariableReference,
    ra_var: UvVariableReference,
    baseline_var: UvVariableReference,
    coord_var: UvVariableReference,
    corr_var: UvVariableReference,

    /// Buffers for quickie data munging.
    corr_buf: Vec<f32>,
    coord_buf: Vec<f64>,
    flag_buf: Vec<bool>,
}

struct Record {
    pub update_time: f64,
    pub coord: Vec<f64>,
    pub corr: Vec<f32>, // XXX shows up as 2*f32, not c64; we don't care
    pub flags: Vec<bool>,
}

impl UvInflator {
    fn process(in_path: &OsStr, out_path: &OsStr) -> Result<i32, Error> {
        let t0 = Instant::now();

        let mut inst = Self::new(in_path, out_path)?;
        inst.mainloop()?;

        let in_mib = inst.in_uv.visdata_bytes() as f64 / (1024. * 1024.);

        inst.out_flags.close()?;
        let out_mib = inst.out_uv.flush(&mut inst.out_ds)? as f64 / (1024. * 1024.);
        inst.out_ds
            .flush()
            .context("couldn't flush changes to output dataset")?;

        let dur = t0.elapsed();
        let dur_secs = dur.subsec_nanos() as f64 * 1e-9 + dur.as_secs() as f64;

        inst.pb.finish();
        println!("elapsed: {:.3} seconds", dur_secs);
        println!(
            "input: {} records, {:.1} MiB => read {:.3} MiB/s",
            inst.in_n,
            in_mib,
            in_mib / dur_secs
        );
        println!(
            "output: {} records, {:.1} MiB => wrote {:.3} MiB/s",
            inst.out_n,
            out_mib,
            out_mib / dur_secs
        );
        Ok(0)
    }

    fn new(in_path: &OsStr, out_path: &OsStr) -> Result<Self, Error> {
        let mut in_ds = DataSet::open(in_path).context("error opening input dataset")?;
        let mut in_uv = in_ds
            .open_uv()
            .context("could not open input as UV dataset")?;
        let mut in_flags = MaskDecoder::new(
            in_ds
                .get("flags")?
                .ok_or(format_err!("no \"flags\" item in input"))?
                .into_byte_stream()?,
        );

        let mut out_ds =
            rubbl_miriad::DataSet::open(out_path).context("error opening output dataset")?;
        let mut out_uv = out_ds
            .new_uv_like(&in_uv)
            .context("could not open output for writing UV data")?;
        let out_flags = MaskEncoder::new(out_ds.create_large_item("flags", Type::Int32)?);

        // Progress bar

        let mut pb = pbr::ProgressBar::new(in_uv.visdata_bytes());
        pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(500)));

        // Get initial complement of variables and copy them over, altering the
        // ones that need it. Namely: antnames, antnums, antpos, cminfo, nants,
        // nbls, nblts, st_type. We *should* skip pol here too, but we're
        // leveraging the fact that we know our datasets are single-pol.

        in_uv.next().context("could not read UV data")?;

        for var in in_uv.variables() {
            match var.name() {
                "coord" | "time" | "baseline" | "corr" | "antnames" | "antnums" | "antpos"
                | "cminfo" | "nants" | "nbls" | "nblts" | "st_type" => {
                    continue;
                }
                _ => {}
            }

            out_uv
                .write_var(var)
                .context("could not write UV variable")?;
        }

        // antnums is an (in_nants)-element f64 array mapping HERA antenna numbers
        // to, uh, correlator input numbers or something? There is probably a less
        // dumb way to assign MIRIAD antnums to the new antennas.

        let mut hera_to_mir = Vec::with_capacity(NANTS);
        let mut antnums_float: Vec<f64> = Vec::with_capacity(NANTS);
        let mut taken_mir_nums = Vec::with_capacity(NANTS);
        taken_mir_nums.resize(NANTS, false);

        in_uv.get_data(
            in_uv
                .lookup_variable("antnums")
                .ok_or(format_err!("no \"antnums\" UV variable"))?,
            &mut antnums_float,
        );

        for antnum_float in &antnums_float {
            let antnum = *antnum_float as usize;
            hera_to_mir.push(antnum);
            taken_mir_nums[antnum] = true;
        }

        let in_nants_live = hera_to_mir.len();
        let in_nants_slots = in_uv.get_scalar::<i32>(
            in_uv
                .lookup_variable("nants")
                .ok_or(format_err!("no \"nants\" UV variable"))?,
        ) as usize;
        let mut last_used_antnum = 0; // assuming antnum 0 is always taken

        for _ in in_nants_live..NANTS {
            while taken_mir_nums[last_used_antnum] {
                last_used_antnum += 1;
            }

            hera_to_mir.push(last_used_antnum);
            antnums_float.push(last_used_antnum as f64);
            taken_mir_nums[last_used_antnum] = true;
        }

        out_uv.write("antnums", &antnums_float)?;

        // Probably a better way to do this all, but I'm distracted.

        let mut mir_to_hera = Vec::with_capacity(NANTS);
        mir_to_hera.resize(NANTS, 0);

        for hera in 0..NANTS {
            mir_to_hera[hera_to_mir[hera]] = hera;
        }

        // We can now build up the mapping from output antnum to input antnum. We
        // just loop around duplicating the existing ones.

        let mut out_ant_to_in = Vec::with_capacity(NANTS);

        for idx in 0..in_nants_live {
            out_ant_to_in.push(idx);
        }

        let n_new = NANTS - in_nants_live;
        let mut in_index = 0;

        println!(
            "numbers: {} live antennas in input set; {} fake antennas being added",
            in_nants_live, n_new
        );

        for _ in 0..n_new {
            out_ant_to_in.push(in_index);
            in_index = (in_index + 1) % in_nants_live;
        }

        // antnames is a string with format "[name, name, name, name, ...]".

        let in_antnames: String = in_uv.get_scalar(
            in_uv
                .lookup_variable("antnames")
                .ok_or(format_err!("no \"antnames\" UV variable"))?,
        );
        let in_antnames = in_antnames.split_at(in_antnames.len() - 2).0.split_at(1).1;
        let mut out_antnames = String::from("[");

        for (idx, name) in in_antnames.split(", ").enumerate() {
            if idx != 0 {
                out_antnames.push_str(", ");
            }

            out_antnames.push_str(name);
        }

        for idx in 0..n_new {
            out_antnames.push_str(&format!(", fake{}", idx));
        }

        out_antnames.push_str("]");
        out_uv.write_scalar("antnames", out_antnames)?;

        // antpos has shape [3,in_nants_slots], where the nants axis is the one
        // that varies fastest. Note that many of the entries are zeros because
        // in_nants_slots = max(heranums) != in_nants_live. Because we're cloning
        // antennas, we fill it out with redundant position entries.

        let mut in_antpos: Vec<f64> = Vec::with_capacity(in_nants_slots * 3);
        in_uv.get_data(
            in_uv
                .lookup_variable("antpos")
                .ok_or(format_err!("no \"antpos\" UV variable"))?,
            &mut in_antpos,
        );

        let mut antpos = Vec::with_capacity(NANTS * 3);
        antpos.resize(NANTS * 3, 0.);

        for heranum in 0..NANTS {
            let src = hera_to_mir[out_ant_to_in[heranum]];
            let dst = hera_to_mir[heranum];

            antpos[dst] = in_antpos[src];
            antpos[dst + NANTS] = in_antpos[src + in_nants_slots];
            antpos[dst + 2 * NANTS] = in_antpos[src + 2 * in_nants_slots];
        }

        out_uv.write("antpos", &antpos)?;

        // cminfo is a JSON struct; not dealing with that.

        out_uv.write_scalar("cminfo", "{'fake': 'dummy'}".to_owned())?;

        // Counts are easy.

        out_uv.write_scalar("nants", NANTS as i32)?;
        let nbls = (NANTS * (NANTS + 1) / 2) as i32; // note, this includes autos
        out_uv.write_scalar("nbls", nbls)?;
        let ntimes: i32 = in_uv.get_scalar(
            in_uv
                .lookup_variable("ntimes")
                .ok_or(format_err!("no \"ntimes\" UV variable"))?,
        );
        out_uv.write_scalar("nblts", nbls * ntimes)?;

        // st_type is formatted the same as antnames.

        let in_st_type: String = in_uv.get_scalar(
            in_uv
                .lookup_variable("st_type")
                .ok_or(format_err!("no \"st_type\" UV variable"))?,
        );
        let in_st_type = in_st_type.split_at(in_st_type.len() - 2).0.split_at(1).1;
        let st_types: Vec<_> = in_st_type.split(", ").collect();

        let mut out_st_type = String::from("[");

        for idx in 0..NANTS {
            if idx != 0 {
                out_st_type.push_str(", ");
            }

            out_st_type.push_str(st_types[out_ant_to_in[idx]]);
        }

        out_st_type.push_str("]");
        out_uv.write_scalar("st_type", out_st_type)?;

        // We're finally ready to actually copy the data! XXX hardcoding single
        // spectral window, no "wide" channels, etc.
        // We've already read the first record, so we need to record it specially.

        let nschan: usize = in_uv.get_scalar::<i32>(
            in_uv
                .lookup_variable("nschan")
                .ok_or(format_err!("no \"nschan\" UV variable"))?,
        ) as usize;

        let time_var = in_uv
            .lookup_variable("time")
            .ok_or(format_err!("no \"time\" UV variable"))?;
        let lst_var = in_uv
            .lookup_variable("lst")
            .ok_or(format_err!("no \"lst\" UV variable"))?;
        let ra_var = in_uv
            .lookup_variable("ra")
            .ok_or(format_err!("no \"ra\" UV variable"))?;
        let baseline_var = in_uv
            .lookup_variable("baseline")
            .ok_or(format_err!("no \"baseline\" UV variable"))?;
        let coord_var = in_uv
            .lookup_variable("coord")
            .ok_or(format_err!("no \"coord\" UV variable"))?;
        let corr_var = in_uv
            .lookup_variable("corr")
            .ok_or(format_err!("no \"corr\" UV variable"))?;

        let time: f64 = in_uv.get_scalar(time_var);
        let lst: f64 = in_uv.get_scalar(lst_var);
        let ra: f64 = in_uv.get_scalar(ra_var);
        let mut records = HashMap::new();

        let bl = decode_baseline(in_uv.get_scalar(baseline_var))?;

        let mut coord = Vec::with_capacity(3);
        in_uv.get_data(coord_var, &mut coord);

        let mut corr = Vec::with_capacity(nschan * 2);
        in_uv.get_data(corr_var, &mut corr);

        let mut flags = Vec::with_capacity(nschan);
        flags.resize(nschan, false);
        in_flags.expand(&mut flags)?;

        records.insert(
            bl,
            Record {
                update_time: time,
                coord: coord,
                corr: corr,
                flags: flags,
            },
        );

        let mut flag_buf = Vec::new();
        flag_buf.resize(nschan, false); // false => bad data

        let mut coord_buf = Vec::new();
        coord_buf.resize(3, 0.);

        Ok(UvInflator {
            pb: pb,

            in_uv: in_uv,
            in_flags: in_flags,
            in_n: 1, // one read already

            out_ds: out_ds,
            out_uv: out_uv,
            out_flags: out_flags,
            out_n: 0,

            hera_to_mir: hera_to_mir,
            mir_to_hera: mir_to_hera,
            out_ant_to_in: out_ant_to_in,
            time: time,
            lst: lst,
            ra: ra,

            records: records,
            time_var: time_var,
            lst_var: lst_var,
            ra_var: ra_var,
            baseline_var: baseline_var,
            coord_var: coord_var,
            corr_var: corr_var,
            corr_buf: Vec::new(),
            coord_buf: coord_buf,
            flag_buf: flag_buf,
        })
    }

    fn mainloop(&mut self) -> Result<(), Error> {
        let mut keep_going = true;

        while keep_going {
            keep_going = self.in_uv.next().context("could not read UV data")?;
            self.in_n += 1;
            let new_time = self.in_uv.get_scalar(self.time_var);
            let cur_time = self.time; // borrowck annoyances

            if new_time != cur_time {
                // Make sure not to update lst and RA until after emitting.
                self.emit(cur_time)?;
                self.time = new_time;
                self.lst = self.in_uv.get_scalar(self.lst_var);
                self.ra = self.in_uv.get_scalar(self.ra_var);
            }

            // XXX should in principle check for updated UV variables beyond
            // the known core.

            let bl = decode_baseline(self.in_uv.get_scalar(self.baseline_var))?;

            if !self.records.contains_key(&bl) {
                let mut flags = Vec::new();
                flags.resize(self.flag_buf.len(), false);

                self.records.insert(
                    bl,
                    Record {
                        update_time: new_time,
                        coord: Vec::new(),
                        corr: Vec::new(),
                        flags: flags,
                    },
                );
            }

            let rec = self.records.get_mut(&bl).unwrap();
            rec.update_time = new_time;
            self.in_uv.get_data(self.coord_var, &mut rec.coord);
            self.in_uv.get_data(self.corr_var, &mut rec.corr);
            self.in_flags.expand(&mut rec.flags)?;
            self.pb.set(self.in_uv.position());
        }

        let t = self.time;
        self.emit(t)
    }

    fn emit(&mut self, time: f64) -> Result<(), Error> {
        self.out_uv.write_scalar("time", time)?;
        self.out_uv.write_scalar("lst", self.lst)?;
        self.out_uv.write_scalar("ra", self.ra)?;

        for ant1 in 0..NANTS {
            for ant2 in ant1..NANTS {
                let hera1 = self.mir_to_hera[ant1];
                let mut src_mir_1 = self.hera_to_mir[self.out_ant_to_in[hera1]];
                let hera2 = self.mir_to_hera[ant2];
                let mut src_mir_2 = self.hera_to_mir[self.out_ant_to_in[hera2]];
                let mut conj = false;

                if src_mir_1 > src_mir_2 {
                    let tmp = src_mir_1;
                    src_mir_1 = src_mir_2;
                    src_mir_2 = tmp;
                    conj = true;
                }

                let rec = self
                    .records
                    .get(&(src_mir_1, src_mir_2))
                    .ok_or(format_err!(
                        "missing data for source {}-{} baseline",
                        src_mir_1,
                        src_mir_2
                    ))?;

                if rec.update_time != time {
                    return Err(format_err!(
                        "stale data for source {}-{} baseline ({:?} vs {:?})",
                        src_mir_1,
                        src_mir_2,
                        rec.update_time,
                        time
                    ));
                }

                // In our fake dataset this is a cross-correlation, but in the
                // source data it is an autocorrelation. Write flagged zeros
                // to avoid funkiness.
                let flag_it = ant1 != ant2 && src_mir_1 == src_mir_2;

                if !conj && !flag_it {
                    self.out_uv.write("corr", &rec.corr)?;
                } else {
                    self.corr_buf.resize(rec.corr.len(), 0.);

                    if !flag_it {
                        self.corr_buf.copy_from_slice(&rec.corr);

                        for i in 0..rec.flags.len() {
                            self.corr_buf[2 * i + 1] *= -1.;
                        }
                    }

                    self.out_uv.write("corr", &self.corr_buf)?;
                }

                if !conj {
                    self.out_uv.write("coord", &rec.coord)?;
                } else {
                    self.coord_buf[0] = -rec.coord[0];
                    self.coord_buf[1] = -rec.coord[1];
                    self.coord_buf[2] = -rec.coord[2];
                    self.out_uv.write("coord", &self.coord_buf)?;
                }

                self.out_uv
                    .write_scalar("baseline", encode_baseline(ant1, ant2)?)?;

                if flag_it {
                    self.out_flags.append_mask(&self.flag_buf)?;
                } else {
                    self.out_flags.append_mask(&rec.flags)?;
                }

                self.out_uv.finish_record()?;
                self.out_n += 1;
            }
        }

        Ok(())
    }
}
