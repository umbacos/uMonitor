"""Read-only, data-only NRBF inspector for testing the independent legacy codec."""
import struct, json, sys
class Reader:
    def __init__(self,data): self.b=data; self.p=0; self.objects={}; self.meta={}; self.libraries={}; self.root=0
    def take(self,n):
        if n<0 or self.p+n>len(self.b): raise ValueError('Truncated data')
        out=self.b[self.p:self.p+n]; self.p+=n; return out
    def u8(self): return self.take(1)[0]
    def i32(self): return struct.unpack('<i',self.take(4))[0]
    def string(self):
        n=0
        for shift in range(0,35,7):
            b=self.u8(); n|=(b&127)<<shift
            if b<128: return self.take(n).decode('utf-8')
        raise ValueError('Bad string length')
    def primitive(self,t):
        if t==1:return bool(self.u8())
        if t==2:return self.u8()
        if t==3:return self.string()
        fmt={5:'d',6:'d',7:'h',8:'i',9:'q',10:'b',11:'f',12:'q',13:'q',14:'H',15:'I',16:'Q'}.get(t)
        if t==18:return self.string()
        if not fmt:raise ValueError('Primitive '+str(t))
        return struct.unpack('<'+fmt,self.take(struct.calcsize(fmt)))[0]
    def additional(self,t):
        if t in (0,7):return self.u8()
        if t==3:return self.string()
        if t==4:return (self.string(),self.i32())
    def value(self,t,a): return self.primitive(a) if t==0 else self.record()
    def record(self):
        tag=self.u8()
        if tag==0:
            self.root=self.i32();self.take(12);return self.record()
        if tag==12:
            k=self.i32();self.libraries[k]=self.string();return self.record()
        if tag in (4,5):
            oid=self.i32();name=self.string();n=self.i32();names=[self.string() for _ in range(n)];types=[self.u8() for _ in range(n)];extra=[self.additional(t) for t in types];lib=self.i32() if tag==5 else 0
            self.meta[oid]=(name,names,types,extra,lib)
            obj={'$type':name,'$id':oid};self.objects[oid]=obj
            for k,t,a in zip(names,types,extra):obj[k]=self.value(t,a)
            return {'$ref':oid}
        if tag==1:
            oid=self.i32();mid=self.i32();name,names,types,extra,lib=self.meta[mid];obj={'$type':name,'$id':oid};self.objects[oid]=obj
            for k,t,a in zip(names,types,extra):obj[k]=self.value(t,a)
            return {'$ref':oid}
        if tag==6:
            oid=self.i32();s=self.string();self.objects[oid]=s;return {'$ref':oid}
        if tag==7:
            oid=self.i32();shape=self.u8();rank=self.i32();lengths=[self.i32() for _ in range(rank)]
            if shape in (3,4,5):[self.i32() for _ in range(rank)]
            t=self.u8();a=self.additional(t);n=1
            for length in lengths:n*=length
            return self.array(oid,n,t,a)
        if tag==8:return self.primitive(self.u8())
        if tag==9:return {'$ref':self.i32()}
        if tag==10:return None
        if tag==11:raise EOFError()
        if tag==13:return {'$nulls':self.u8()}
        if tag==14:return {'$nulls':self.i32()}
        if tag==15:
            oid=self.i32();n=self.i32();t=self.u8();return self.array(oid,n,0,t)
        if tag in (16,17):return self.array(self.i32(),self.i32(),2,None)
        raise ValueError('Record '+str(tag)+' at '+str(self.p-1))
    def array(self,oid,n,t,a):
        if n>4000000 or n<0:raise ValueError('Invalid array')
        arr=[];self.objects[oid]=arr
        while len(arr)<n:
            v=self.value(t,a)
            if isinstance(v,dict) and '$nulls'in v:arr.extend([None]*v['$nulls'])
            else:arr.append(v)
        return {'$ref':oid}
    def all(self):
        try:
            while True:self.record()
        except EOFError:pass
        return self
    def resolve(self,v):
        for _ in range(20):
            if isinstance(v,dict) and '$ref'in v:v=self.objects.get(v['$ref'])
            else:return v
        raise ValueError('Reference loop')
if __name__=='__main__':
    for path in sys.argv[1:]:
        r=Reader(open(path,'rb').read()).all()
        print(path,'objects',len(r.objects),'bytes read',r.p)
        for k,v in r.objects.items():
            if isinstance(v,dict) and any(t in v.get('$type','') for t in ['ThemeConfig','TGroup','TItem','ScreenTip','FontConfig','StatuBar','ArchBar','Bitmap']):
                print(k,json.dumps(v,ensure_ascii=True)[:2400])
