"""Text checks and whole-document rendering; visual review remains explicit."""
from pathlib import Path
import subprocess, hashlib, json, re
from pypdf import PdfReader
from PIL import Image
ROOT=Path(__file__).resolve().parents[2]
def main():
    paper=ROOT/'paper';pdf=ROOT/'QENLO-RESEARCH-PAPER.pdf';out=paper/'tmp/final-pages'
    out.mkdir(parents=True,exist_ok=True)
    text_path=paper/'audit/final-text.txt'
    subprocess.run(['pdftotext','-layout',str(pdf),str(text_path)],check=True)
    text=text_path.read_text(encoding='utf-8')
    expected=['9.409','0.897','14.800','15.343','18.695','0.671','28.599','0.896','3.055','6.202','2.936','0.384','101.296','52.169','23.9','235.7','8 of 16','17.0','28.0','26.9','0.9400778694252952','References','FP64','227 of 260','614,400','1,152,000','0.99998','424,640','6,794,240','1.673','e4ca1336']
    missing=[s for s in expected if s not in text]
    assert not missing,missing
    assert '??' not in text and '\ufffd' not in text,'Missing or corrupt text marker'
    reader=PdfReader(pdf)
    pages=[]
    for i,p in enumerate(reader.pages,1):
        content=p.extract_text() or ''
        assert len(content.strip())>100,f'Unexpectedly blank page {i}'
        pages.append({'page':i,'text_characters':len(content)})
    assert len(set(re.findall(r'Figure (\d+):',text)))==14,'Missing figure captions'
    assert len(set(re.findall(r'\[(\d+)\]',text)))>=15,'Missing numbered bibliography entries'
    subprocess.run(['pdftoppm','-png','-r','120',str(pdf),str(out/'page')],check=True)
    pdf_sha256=hashlib.sha256(pdf.read_bytes()).hexdigest()
    review_path=paper/'audit/visual-review-2026-09-14.json'
    review=json.loads(review_path.read_text(encoding='utf-8')) if review_path.exists() else None
    reviewed=bool(review and review.get('status')=='passed' and review.get('pdf_sha256')==pdf_sha256 and review.get('page_count')==len(pages))
    for record in pages:
        p=out/f"page-{record['page']:02d}.png"
        assert p.exists()
        with Image.open(p) as im:
            record['render_size']=list(im.size)
            record['pixel_sha256']=hashlib.sha256(im.convert('RGB').tobytes()).hexdigest()
        record['render_path']=str(p.relative_to(ROOT)).replace('\\','/')
        record['visual_status']='passed by matching visual review record' if reviewed else 'pending'
    result={'pdf_sha256':pdf_sha256,'page_count':len(pages),'text_checks_passed':True,'expected_values':expected,'rendered_all_pages':True,'pages':pages,'visual_status':'passed' if reviewed else 'pending human/model image inspection','visual_review_record':str(review_path.relative_to(ROOT)).replace('\\','/') if reviewed else None}
    (paper/'audit/pdf-text-render-verification.json').write_text(json.dumps(result,indent=2)+'\n',newline='\r\n')
    print(f"Text checks passed; rendered {len(pages)} pages. Visual review {'passed' if reviewed else 'pending'}.")
if __name__=='__main__':main()
