# Generates casatables/tests/data/dysco.tab with python-casacore (>=3.6, bundles Dysco).
import numpy as np, casacore.tables as pt, shutil, sys
out=sys.argv[1]; shutil.rmtree(out,ignore_errors=True)
nant=4; ntime=3; nchan=8; npol=4
a1,a2=np.triu_indices(nant,1); nbl=len(a1); nrow=nbl*ntime
sh=[nchan,npol]
cols=[pt.makescacoldesc("ANTENNA1",0),pt.makescacoldesc("ANTENNA2",0),pt.makescacoldesc("FIELD_ID",0),
      pt.makescacoldesc("DATA_DESC_ID",0),pt.makescacoldesc("TIME",0.0),
      pt.makearrcoldesc("DATA",0j,valuetype="complex",shape=sh,options=5),
      pt.makearrcoldesc("WEIGHT_SPECTRUM",0.0,valuetype="float",shape=sh,options=5),
      pt.makearrcoldesc("DATA_ORIG",0j,valuetype="complex",shape=sh,options=5),
      pt.makearrcoldesc("DATA_DECODED",0j,valuetype="complex",shape=sh,options=5),
      pt.makearrcoldesc("WEIGHT_DECODED",0.0,valuetype="float",shape=sh,options=5)]
dm={"*1":{"TYPE":"DyscoStMan","NAME":"DyscoData","COLUMNS":["DATA","WEIGHT_SPECTRUM"],
     "SPEC":{"dataBitCount":10,"weightBitCount":12,"distribution":"TruncatedGaussian",
             "distributionTruncation":2.5,"normalization":"AF","studentTNu":0.0,"staticSeed":True}}}
t=pt.table(out,pt.maketabdesc(cols),nrow=nrow,dminfo=dm,ack=False)
t.putcol("ANTENNA1",np.tile(a1,ntime)); t.putcol("ANTENNA2",np.tile(a2,ntime))
t.putcol("TIME",np.repeat(5e9+8.0*np.arange(ntime),nbl))
rng=np.random.default_rng(1)
d=(rng.standard_normal((nrow,*sh))+1j*rng.standard_normal((nrow,*sh))).astype(np.complex64)
w=np.repeat(rng.uniform(0.5,2,(nrow,1,1)),nchan,1).repeat(npol,2).astype(np.float32)
t.putcol("DATA",d); t.putcol("WEIGHT_SPECTRUM",w); t.putcol("DATA_ORIG",d)
t.close()
t=pt.table(out,readonly=False,ack=False)
t.putcol("DATA_DECODED",t.getcol("DATA")); t.putcol("WEIGHT_DECODED",t.getcol("WEIGHT_SPECTRUM"))
print(t.getdminfo("DATA")["TYPE"], t.getdminfo("DATA_ORIG")["TYPE"])
t.close()
