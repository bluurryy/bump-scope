inspect_asm::bump_vec_u32::down::try_push:
	push r14
	push rbx
	push rax
	mov rax, qword ptr [rdi + 8]
	cmp qword ptr [rdi + 16], rax
	je .LBB_1
.LBB_3:
	mov rcx, qword ptr [rdi]
	mov dword ptr [rcx + 4*rax], esi
	inc rax
	mov qword ptr [rdi + 8], rax
	xor eax, eax
.LBB_4:
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_1:
	mov ebx, esi
	mov r14, rdi
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	mov ecx, eax
	mov al, 1
	test cl, cl
	jne .LBB_4
	mov rdi, r14
	mov rax, qword ptr [r14 + 8]
	mov esi, ebx
	jmp .LBB_3