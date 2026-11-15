"""Read delivered text, outline titles and destinations through the system PDF reader."""
import ctypes as c
import ctypes.util
from common import require

INPUTS = ['tools/paper/pdf_text.py']


class Pdf:
    def __init__(self, path):
        self.lib = c.CDLL(ctypes.util.find_library('poppler-glib'))
        self.glib = c.CDLL(ctypes.util.find_library('glib-2.0'))
        self.object = c.CDLL(ctypes.util.find_library('gobject-2.0'))
        self.open = self.bind('poppler_document_new_from_file', c.c_void_p, c.c_char_p, c.c_char_p, c.c_void_p)
        self.doc = self.open(path.resolve().as_uri().encode('utf-8'), None, None)
        require(self.doc, 'Cannot read PDF: '+str(path))
        self.count = self.bind('poppler_document_get_n_pages', c.c_int, c.c_void_p)(self.doc)
        self.get_page = self.bind('poppler_document_get_page', c.c_void_p, c.c_void_p, c.c_int)
        self.get_text = self.bind('poppler_page_get_text', c.c_void_p, c.c_void_p)
        class Rectangle(c.Structure):
            _fields_ = [('x1',c.c_double),('y1',c.c_double),('x2',c.c_double),('y2',c.c_double)]
        self.rectangle = Rectangle
        self.layout = self.bind('poppler_page_get_text_layout',c.c_int,c.c_void_p,
                                c.POINTER(c.POINTER(Rectangle)),c.POINTER(c.c_uint))
        self.find = self.bind('poppler_document_find_dest', c.c_void_p, c.c_void_p, c.c_char_p)
        self.dest_free = self.bind('poppler_dest_free', None, c.c_void_p)
        self.free = self.glib.g_free
        self.free.argtypes = [c.c_void_p]
        self.unref = self.object.g_object_unref
        self.unref.argtypes = [c.c_void_p]

    def bind(self, name, result, *args):
        fn = getattr(self.lib,name); fn.restype=result; fn.argtypes=list(args)
        return fn

    def pages(self):
        result = []
        for n in range(self.count):
            page = self.get_page(self.doc,n)
            text = self.get_text(page)
            decoded=c.string_at(text).decode('utf-8')
            areas=c.POINTER(self.rectangle)(); count=c.c_uint()
            require(self.layout(page,c.byref(areas),c.byref(count)), 'PDF text layout unavailable')
            require(count.value==len(decoded), 'PDF text/layout character counts differ')
            spaced=[]
            for i,ch in enumerate(decoded):
                separated = i and (areas[i].x1-areas[i-1].x2>3.0 or
                                   areas[i].x1<areas[i-1].x1-3.0 or
                                   abs(areas[i].y1-areas[i-1].y1)>3.0 or
                                   abs((areas[i].y2-areas[i].y1)-(areas[i-1].y2-areas[i-1].y1))>0.5)
                if separated and not decoded[i-1].isspace() and not ch.isspace():
                    spaced.append(' ')
                spaced.append(ch)
            result.append(''.join(spaced))
            self.free(areas)
            self.free(text); self.unref(page)
        return result

    def destination(self, name):
        dest = self.find(self.doc,name.encode('utf-8'))
        if not dest: return None
        # PopplerDest begins with type and its one-based page number.
        page = c.cast(dest,c.POINTER(c.c_int))[1]
        self.dest_free(dest)
        return page

    def bookmarks(self):
        new = self.bind('poppler_index_iter_new',c.c_void_p,c.c_void_p)
        child = self.bind('poppler_index_iter_get_child',c.c_void_p,c.c_void_p)
        advance = self.bind('poppler_index_iter_next',c.c_int,c.c_void_p)
        action = self.bind('poppler_index_iter_get_action',c.c_void_p,c.c_void_p)
        free_action = self.bind('poppler_action_free',None,c.c_void_p)
        free_iter = self.bind('poppler_index_iter_free',None,c.c_void_p)
        class AnyAction(c.Structure):
            _fields_ = [('kind',c.c_int),('title',c.c_char_p)]
        class Destination(c.Structure):
            _fields_ = [('kind',c.c_int),('page',c.c_int),('left',c.c_double),('bottom',c.c_double),
                        ('right',c.c_double),('top',c.c_double),('zoom',c.c_double),('name',c.c_char_p)]
        class GotoAction(c.Structure):
            _fields_ = [('kind',c.c_int),('title',c.c_char_p),('destination',c.POINTER(Destination))]
        out = []
        def walk(it):
            if not it: return
            while True:
                a = action(it)
                if a:
                    title = c.cast(a,c.POINTER(AnyAction)).contents.title
                    if title:
                        item=dict(title=title.decode('utf-8'),page=None,anchor='')
                        if c.cast(a,c.POINTER(AnyAction)).contents.kind==2:
                            dest=c.cast(a,c.POINTER(GotoAction)).contents.destination.contents
                            if dest.kind==9 and dest.name:
                                item['anchor']=dest.name.decode('utf-8'); item['page']=self.destination(item['anchor'])
                            else:item['page']=dest.page
                        out.append(item)
                    free_action(a)
                walk(child(it))
                if not advance(it): break
            free_iter(it)
        walk(new(self.doc))
        return out
