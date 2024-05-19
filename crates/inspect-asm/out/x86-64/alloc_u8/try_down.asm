inspect_asm::alloc_u8::try_down:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	dec rax
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_2
	mov qword ptr [rcx], rax
	test rax, rax
	je .LBB_2
.LBB_4:
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB_2:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	test rax, rax
	jne .LBB_4
	xor eax, eax
	pop rbx
	ret